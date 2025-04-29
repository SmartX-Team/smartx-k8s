use std::{collections::BTreeMap, fmt, hash::Hash, sync::Arc, time::Duration};

use anyhow::Result;
use chrono::Utc;
use futures::StreamExt;
use k8s_openapi::{
    NamespaceResourceScope,
    api::core::v1::Service,
    apimachinery::pkg::apis::meta::v1::{Condition, OwnerReference, Time},
    serde::{Serialize, de::DeserializeOwned},
};
use kube::{
    Api, Client, CustomResourceExt, Error, Resource as _, ResourceExt,
    api::{ApiResource, ObjectMeta, Patch, PatchParams, PostParams, ValidationDirective},
    runtime::{
        Controller,
        controller::Action,
        events::{Recorder, Reporter},
        reflector::ObjectRef,
        watcher::Config,
    },
};
use openark_core::operator::RecorderExt;
use openark_traffic_api::{
    http_route_claim::{RouteResource, RouteResourceBackendRef, RouteResourceLifecycle},
    traffic_route_claim::RouteClaimStatus,
    traffic_router_class::TrafficRouterClassCrd,
};
use serde_json::json;
#[cfg(feature = "tracing")]
use tracing::{Level, info, instrument};

pub(crate) trait TrafficRouteClaim
where
    Self: Clone + fmt::Debug + DeserializeOwned + ::kube::Resource<Scope = NamespaceResourceScope>,
{
    type Target: Send
        + Sync
        + Clone
        + fmt::Debug
        + Serialize
        + DeserializeOwned
        + ::kube::Resource<Scope = NamespaceResourceScope>;
    type TargetSpec: Clone + Serialize;

    fn build_target(
        metadata: ObjectMeta,
        spec: <Self as TrafficRouteClaim>::TargetSpec,
    ) -> <Self as TrafficRouteClaim>::Target;

    fn metadata(&self) -> &ObjectMeta;

    fn class_name(&self) -> &str;

    fn resources(&self) -> &[RouteResource];

    fn template(&self) -> &<Self as TrafficRouteClaim>::TargetSpec;
}

enum FailbackAction {
    AwaitChange,
    Requeue,
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
async fn get_class<F, Fut>(
    api: &Api<TrafficRouterClassCrd>,
    name: &str,
    update_status: F,
) -> Result<Result<TrafficRouterClassCrd, FailbackAction>, Error>
where
    F: Copy + Fn(String, String) -> Fut,
    Fut: Future<Output = Result<(), Error>>,
{
    if name.trim().is_empty() {
        let reason = "InvalidClass".into();
        let message = format!("Empty class");
        update_status(reason, message).await?;
        return Ok(Err(FailbackAction::AwaitChange));
    }

    match api.get_opt(name).await? {
        Some(class) => {
            if class
                .status
                .as_ref()
                .is_some_and(|status| status.is_accepted())
            {
                Ok(Ok(class))
            } else {
                let reason = "Pending".into();
                let message = format!("Class is not accepted: {name}");
                update_status(reason, message).await?;
                return Ok(Err(FailbackAction::Requeue));
            }
        }
        None => {
            let reason = "InvalidClass".into();
            let message = format!("Class not found: {name}");
            update_status(reason, message).await?;
            return Ok(Err(FailbackAction::Requeue));
        }
    }
}

struct Resource<'a> {
    spec: ResourceSpec,
    lifecycle: &'a RouteResourceLifecycle,
}

enum ResourceSpec {
    Service(Service),
}

impl ResourceSpec {
    fn owner_ref(&self) -> Option<OwnerReference> {
        match self {
            Self::Service(res) => res.owner_ref(&()),
        }
    }
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
async fn get_resource<'a, K, F, Fut>(
    claim: &'a K,
    ctx: &Context,
    res: &'a RouteResource,
    update_status: F,
) -> Result<Result<Resource<'a>, FailbackAction>, Error>
where
    K: TrafficRouteClaim,
    F: Fn(String, String) -> Fut,
    Fut: Future<Output = Result<(), Error>>,
{
    let group = res.backend_ref.object.group.as_str();
    let kind = res.backend_ref.object.kind.as_str();
    let name = res.backend_ref.object.name.as_str();
    let namespace = res
        .backend_ref
        .object
        .namespace
        .as_deref()
        .or(claim.metadata().namespace.as_deref())
        .expect("Namespaced resource");

    let spec = match (group, kind) {
        ("", "Service") => {
            let api = Api::namespaced(ctx.client.clone(), namespace);
            match api.get_opt(name).await? {
                Some(spec) => ResourceSpec::Service(spec),
                None => {
                    let reason = "InvalidResources".into();
                    let message = if group.is_empty() {
                        format!("Resource not found: {kind}/{namespace}/{name}")
                    } else {
                        format!("Resource not found: {group}/{kind}/{namespace}/{name}")
                    };
                    update_status(reason, message).await?;
                    return Ok(Err(FailbackAction::Requeue));
                }
            }
        }
        (group, kind) => {
            let reason = "InvalidResources".into();
            let message = if group.is_empty() {
                format!("Unsupported resource kind: {kind}")
            } else {
                format!("Unsupported resource kind: {group}/{kind}")
            };
            update_status(reason, message).await?;
            return Ok(Err(FailbackAction::AwaitChange));
        }
    };
    Ok(Ok(Resource {
        spec,
        lifecycle: &res.lifecycle,
    }))
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
async fn get_resources<'a, K, F, Fut>(
    claim: &'a K,
    ctx: &Context,
    update_status: F,
) -> Result<Result<BTreeMap<&'a RouteResourceBackendRef, Resource<'a>>, FailbackAction>, Error>
where
    K: TrafficRouteClaim,
    F: Copy + Fn(String, String) -> Fut,
    Fut: Future<Output = Result<(), Error>>,
{
    let mut map = BTreeMap::default();
    for res in claim.resources() {
        match get_resource(claim, &ctx, res, update_status).await? {
            Ok(resource) => {
                if map.insert(&res.backend_ref, resource).is_some() {
                    let group = res.backend_ref.object.group.as_str();
                    let kind = res.backend_ref.object.kind.as_str();
                    let name = res.backend_ref.object.name.as_str();

                    let reason = "InvalidResources".into();
                    let message = if group.is_empty() {
                        format!("Duplicaed resource: {kind}/{name}")
                    } else {
                        format!("Duplicaed resource: {group}/{kind}/{name}")
                    };
                    update_status(reason, message).await?;
                    return Ok(Err(FailbackAction::AwaitChange));
                }
            }
            Err(action) => return Ok(Err(action)),
        }
    }
    Ok(Ok(map))
}

struct Context {
    api_class: Api<TrafficRouterClassCrd>,
    client: Client,
    controller_name: String,
    crd: ApiResource,
    crd_target: ApiResource,
    interval: Duration,
    namespace: Option<String>,
    patch_params: PatchParams,
    post_params: PostParams,
    recorder: Recorder,
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
async fn reconcile<K>(claim: Arc<K>, ctx: Arc<Context>) -> Result<Action, Error>
where
    K: TrafficRouteClaim,
    <K as ::kube::Resource>::DynamicType: Default,
    <<K as TrafficRouteClaim>::Target as ::kube::Resource>::DynamicType: Default,
{
    let metadata = claim.metadata();
    let name = claim.name_any();
    let namespace = metadata.namespace.as_deref().expect("Namespaced resource");

    // Define fallback action converter
    let convert_action = |action| match action {
        FailbackAction::AwaitChange => Action::await_change(),
        FailbackAction::Requeue => Action::requeue(ctx.interval),
    };

    // Define status update function
    let update_status = |reason: String, message: String| async {
        let patch = Patch::Merge(json!({
            "apiVersion": ctx.crd.api_version,
            "kind": ctx.crd.kind,
            "status": RouteClaimStatus {
                conditions: vec![
                    Condition {
                        last_transition_time: Time(Utc::now()),
                        message: message.clone(),
                        observed_generation: metadata.generation,
                        reason: reason.clone(),
                        status: match reason.as_str() {
                            "Accepted" => "True".into(),
                            _ => "False".into(),
                        },
                        type_: "Accepted".into(),
                    },
                ],
            },
        }));

        let api = Api::<K>::namespaced(ctx.client.clone(), namespace);
        api.patch_status(&name, &ctx.patch_params, &patch).await?;

        report_update(&ctx.recorder, &*claim, reason, message).await
    };

    // Validate class
    let class = match get_class(&ctx.api_class, &claim.class_name(), &update_status).await? {
        Ok(class) => class,
        Err(action) => return Ok(convert_action(action)),
    };

    // Validate resources
    let resources = match get_resources(&*claim, &ctx, &update_status).await? {
        Ok(list) => list,
        Err(action) => return Ok(convert_action(action)),
    };

    // Validate template
    let spec = claim.template().clone();

    // Build owner references
    let mut maybe_owner_references = vec![
        claim.owner_ref(&Default::default()),
        class.owner_ref(&Default::default()),
    ];
    for resource in resources.values() {
        maybe_owner_references.push(resource.spec.owner_ref());
    }
    let owner_references = maybe_owner_references
        .into_iter()
        .flatten()
        .map(|mut owner_ref| {
            owner_ref.block_owner_deletion = Some(true);
            owner_ref
        })
        .collect();

    // Create/Update a target resource
    {
        let api = Api::namespaced(ctx.client.clone(), namespace);
        let metadata = ObjectMeta {
            name: Some(name.clone()),
            namespace: Some(namespace.into()),
            owner_references: Some(owner_references),
            ..Default::default()
        };
        let target = <K as TrafficRouteClaim>::build_target(metadata, spec);
        match api.get_metadata_opt(&name).await? {
            Some(_) => api.replace(&name, &ctx.post_params, &target).await?,
            None => api.create(&ctx.post_params, &target).await?,
        };
    }

    {
        let reason = "Accepted".into();
        let message = format!("Valid {}", &ctx.crd.kind);
        update_status(reason, message).await?;
        Ok(Action::await_change())
    }
}

async fn report_update<K>(
    recorder: &Recorder,
    claim: &K,
    reason: String,
    message: String,
) -> Result<(), ::kube::Error>
where
    K: TrafficRouteClaim,
    <K as ::kube::Resource>::DynamicType: Default,
{
    let event = ::kube::runtime::events::Event {
        type_: match reason.as_str() {
            "Accepted" => ::kube::runtime::events::EventType::Normal,
            _ => ::kube::runtime::events::EventType::Warning,
        },
        reason,
        note: Some(message),
        action: "Accepted".into(),
        secondary: None,
    };
    let reference = ObjectRef::from_obj(claim).into();
    recorder.report_update(&event, &reference).await
}

async fn report_error(
    recorder: &Recorder,
    error: ::kube::runtime::controller::Error<Error, ::kube::runtime::watcher::Error>,
) {
    let reason = "Failed".into();
    let action = "Accepted".into();
    recorder.report_error(error, reason, action).await
}

fn error_policy<K>(_claim: Arc<K>, _error: &Error, ctx: Arc<Context>) -> Action {
    Action::requeue(ctx.interval)
}

pub(crate) async fn loop_forever<K>(args: super::Args, client: Client) -> Result<()>
where
    K: 'static + Send + Sync + CustomResourceExt + TrafficRouteClaim,
    <K as ::kube::Resource>::DynamicType: Unpin + Clone + fmt::Debug + Default + Eq + Hash,
    <K as TrafficRouteClaim>::Target: CustomResourceExt,
    <<K as TrafficRouteClaim>::Target as ::kube::Resource>::DynamicType: Default,
{
    let api: Api<K> = match args.operator.namespace.as_deref() {
        Some(ns) => Api::namespaced(client.clone(), ns),
        None => Api::all(client.clone()),
    };
    let api_class = Api::all(client.clone());

    let patch_params = PatchParams {
        dry_run: false,
        force: false,
        field_manager: Some(args.operator.controller_name.clone()),
        field_validation: Some(ValidationDirective::Strict),
    };
    let post_params = PostParams {
        dry_run: false,
        field_manager: Some(args.operator.controller_name.clone()),
    };

    let reporter = Reporter {
        controller: args.operator.controller_name.clone(),
        instance: args.operator.controller_pod_name.clone(),
    };
    let recorder = Recorder::new(client.clone(), reporter);

    let watcher_config = Config::default();

    let context = Arc::new(Context {
        api_class,
        client,
        controller_name: args.operator.controller_name.clone(),
        crd: K::api_resource(),
        crd_target: <<K as TrafficRouteClaim>::Target as CustomResourceExt>::api_resource(),
        interval: Duration::from_secs(30),
        namespace: args.operator.namespace.clone(),
        patch_params,
        post_params,
        recorder: recorder.clone(),
    });

    Controller::new(api, watcher_config)
        .run(reconcile, error_policy, context)
        .for_each(|res| async {
            match res {
                Ok((object, _)) => {
                    #[cfg(feature = "tracing")]
                    info!("reconciled {object:?}");
                    let _ = object;
                }
                Err(error) => report_error(&recorder, error).await,
            }
        })
        .await;
    Ok(())
}
