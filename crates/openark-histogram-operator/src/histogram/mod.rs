mod service;

use std::{sync::Arc, time::Duration};

use anyhow::Result;
use chrono::Utc;
use futures::StreamExt;
use k8s_openapi::{
    Resource as _,
    api::core::v1::{Service, ServiceSpec},
    apimachinery::pkg::apis::meta::v1::{Condition, OwnerReference, Time},
};
use kube::{
    Api, Client, CustomResourceExt, Error, Resource, ResourceExt,
    api::{ApiResource, ObjectMeta, Patch, PatchParams, PostParams, ValidationDirective},
    runtime::{
        Controller,
        controller::Action,
        events::{Recorder, Reporter},
        reflector::ObjectRef,
        watcher::Config,
    },
};
use openark_core::operator::{RecorderExt, is_conditions_changed};
use openark_histogram_api::{
    common::ObjectReference,
    histogram::{HistogramCrd, HistogramStatus},
    histogram_class::HistogramClassCrd,
};
use serde_json::json;
#[cfg(feature = "tracing")]
use tracing::{Level, info, instrument};

struct ValidationError {
    reason: String,
    message: String,
    requeue: bool,
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
async fn get_class(
    api: &Api<HistogramClassCrd>,
    name: &str,
) -> Result<Result<HistogramClassCrd, ValidationError>, Error> {
    if name.trim().is_empty() {
        return Ok(Err(ValidationError {
            reason: "InvalidClass".into(),
            message: "Empty class".into(),
            requeue: false,
        }));
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
                return Ok(Err(ValidationError {
                    reason: "Pending".into(),
                    message: format!("Class is not accepted: {name}"),
                    requeue: true,
                }));
            }
        }
        None => {
            return Ok(Err(ValidationError {
                reason: "InvalidClass".into(),
                message: format!("Class not found: {name}"),
                requeue: true,
            }));
        }
    }
}

enum HistogramTarget {
    Service(Option<ServiceSpec>),
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
async fn get_target(
    hist: &HistogramCrd,
    ctx: &Context,
) -> Result<Result<(ObjectMeta, HistogramTarget), ValidationError>, Error> {
    let ObjectReference {
        group,
        kind,
        name,
        namespace,
    } = &hist.spec.target_ref;

    let namespace = namespace
        .as_deref()
        .or(hist.metadata.namespace.as_deref())
        .expect("namespaced resource");

    match (group.as_str(), kind.as_str()) {
        (Service::GROUP, Service::KIND) => {
            let api = Api::<Service>::namespaced(ctx.kube.clone(), namespace);
            match api.get_opt(name).await? {
                Some(item) => Ok(Ok((item.metadata, HistogramTarget::Service(item.spec)))),
                None => Ok(Err(ValidationError {
                    reason: "InvalidTarget".into(),
                    message: format!("Target not found: {kind}/{namespace}/{name}"),
                    requeue: true,
                })),
            }
        }
        (group, kind) => Ok(Err(ValidationError {
            reason: "InvalidTarget".into(),
            message: if group.is_empty() {
                format!("Unsupported target kind: {kind}")
            } else {
                format!("Unsupported target kind: {group}/{kind}")
            },
            requeue: false,
        })),
    }
}

struct Context {
    api_class: Api<HistogramClassCrd>,
    client: ::reqwest::Client,
    crd: ApiResource,
    interval: Duration,
    kube: Client,
    label_parent: String,
    patch_params: PatchParams,
    post_params: PostParams,
    recorder: Recorder,
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
async fn reconcile(hist: Arc<HistogramCrd>, ctx: Arc<Context>) -> Result<Action, Error> {
    let metadata = &hist.metadata;
    let name = hist.name_any();
    let namespace = metadata.namespace.as_deref().expect("Namespaced resource");

    // Define status update function
    let update_status = |reason: String, message: String| async {
        let status = HistogramStatus {
            conditions: vec![Condition {
                last_transition_time: Time(Utc::now()),
                message: message.clone(),
                observed_generation: metadata.generation,
                reason: reason.clone(),
                status: match reason.as_str() {
                    "Accepted" => "True".into(),
                    _ => "False".into(),
                },
                type_: "Accepted".into(),
            }],
        };

        // Skip updating status if nothing has been changed
        let has_changed = hist
            .status
            .as_ref()
            .is_none_or(|HistogramStatus { conditions }| {
                is_conditions_changed(conditions, &status.conditions)
            });

        if has_changed {
            let patch = Patch::Merge(json!({
                "apiVersion": ctx.crd.api_version,
                "kind": ctx.crd.kind,
                "status": status,
            }));

            let api = Api::<HistogramCrd>::namespaced(ctx.kube.clone(), namespace);
            api.patch_status(&name, &ctx.patch_params, &patch).await?;
        }

        report_update(&ctx.recorder, &*hist, reason, message).await
    };

    // Validate class
    let class = match get_class(&ctx.api_class, &hist.spec.histogram_class_name).await? {
        Ok(class) => class,
        Err(action) => {
            update_status(action.reason, action.message).await?;
            return if action.requeue {
                Ok(Action::requeue(ctx.interval))
            } else {
                Ok(Action::await_change())
            };
        }
    };

    // Validate target
    let (target_metadata, target_spec) = match get_target(&*hist, &ctx).await? {
        Ok(list) => list,
        Err(action) => {
            update_status(action.reason, action.message).await?;
            return if action.requeue {
                Ok(Action::requeue(ctx.interval))
            } else {
                Ok(Action::await_change())
            };
        }
    };

    // Build owner references
    let child_owner_references = vec![OwnerReference {
        block_owner_deletion: Some(true),
        controller: Some(false),
        ..hist
            .owner_ref(&Default::default())
            .expect("histogram owner reference")
    }];

    // Build metadata template
    let child_metadata = ObjectMeta {
        namespace: Some(namespace.into()),
        annotations: {
            let mut map = target_metadata.annotations.clone().unwrap_or_default();
            if let Some(mut hist_map) = metadata.annotations.clone() {
                map.append(&mut hist_map);
            }
            Some(map)
        },
        labels: {
            let mut map = target_metadata.labels.clone().unwrap_or_default();
            if let Some(mut hist_map) = metadata.labels.clone() {
                map.append(&mut hist_map);
            }
            map.insert(ctx.label_parent.clone(), name.clone());
            Some(map)
        },
        owner_references: Some(child_owner_references),
        ..Default::default()
    };

    // Create/Update a target resource
    match match target_spec {
        HistogramTarget::Service(spec) => {
            let child_ctx = self::service::Context {
                address_type: "IPv4",
                child_metadata: &child_metadata,
                class: &class,
                client: &ctx.client,
                kube: &ctx.kube,
                label_parent: &ctx.label_parent,
                name: &name,
                namespace: &namespace,
                post_params: &ctx.post_params,
                settings: &hist.spec.histogram,
                target_metadata: &target_metadata,
                target_spec: spec.as_ref(),
            };
            self::service::update_subresources(child_ctx).await?
        }
    } {
        Ok(()) => (),
        Err(action) => {
            update_status(action.reason, action.message).await?;
            return if action.requeue {
                Ok(Action::requeue(ctx.interval))
            } else {
                Ok(Action::await_change())
            };
        }
    }

    // Completed provisioning
    {
        let reason = "Accepted".into();
        let message = format!("Valid {}", &ctx.crd.kind);
        update_status(reason, message).await?;

        let interval = hist
            .spec
            .histogram
            .interval
            .map(Duration::from_secs)
            .unwrap_or(ctx.interval);
        Ok(Action::requeue(interval))
    }
}

async fn report_update(
    recorder: &Recorder,
    hist: &HistogramCrd,
    reason: String,
    message: String,
) -> Result<(), ::kube::Error> {
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
    let reference = ObjectRef::from_obj(hist).into();
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

fn error_policy<K>(_hist: Arc<K>, _error: &Error, ctx: Arc<Context>) -> Action {
    Action::requeue(ctx.interval)
}

pub(crate) async fn loop_forever(
    args: super::Args,
    client: ::reqwest::Client,
    kube: Client,
) -> Result<()> {
    let api: Api<HistogramCrd> = match args.operator.namespace.as_deref() {
        Some(ns) => Api::namespaced(kube.clone(), ns),
        None => Api::all(kube.clone()),
    };
    let api_class = Api::all(kube.clone());

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
    let recorder = Recorder::new(kube.clone(), reporter);

    let watcher_config = Config::default();

    let context = Arc::new(Context {
        api_class,
        client,
        crd: HistogramCrd::api_resource(),
        interval: Duration::from_secs(30),
        label_parent: args.label_parent,
        kube,
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
