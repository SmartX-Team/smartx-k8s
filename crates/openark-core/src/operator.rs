use std::time::Duration;

use async_trait::async_trait;
#[cfg(feature = "clap")]
use clap::Parser;
use jiff::Timestamp;
use k8s_openapi::{
    api::core::v1::ObjectReference,
    apimachinery::pkg::apis::meta::v1::{Condition, ObjectMeta, Time},
};
use kube::{
    Api, Client, CustomResourceExt, Result,
    api::{Patch, PatchParams, PostParams},
    runtime::{
        controller::Action,
        events::{Event, EventType, Recorder},
        reflector::{Lookup, ObjectRef},
    },
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;
#[cfg(feature = "tracing")]
use tracing::{Level, error, info, instrument};

#[derive(Clone, Debug)]
#[cfg_attr(feature = "clap", derive(Parser))]
#[cfg_attr(feature = "clap", command(author, version, about, long_about = None))]
pub struct OperatorArgs {
    /// The current controller's 'name.
    #[cfg_attr(feature = "clap", arg(long, env = "CONTROLLER_NAME"))]
    pub controller_name: String,

    /// The current controller pod's name. Can be opted.
    #[cfg_attr(feature = "clap", arg(long, env = "CONTROLLER_POD_NAME"))]
    pub controller_pod_name: Option<String>,

    /// Whether to install CRDs.
    #[cfg_attr(feature = "clap", arg(long, env = "INSTALL_CRDS"))]
    pub install_crds: bool,

    /// Target namespace.
    #[cfg_attr(feature = "clap", arg(long, env = "NAMESPACE", value_name = "NAME"))]
    pub namespace: Option<String>,

    /// Whether to upgrade CRDs.
    #[cfg_attr(feature = "clap", arg(long, env = "UPGRADE_CRDS"))]
    pub upgrade_crds: bool,
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
pub async fn install_crd<K>(args: &OperatorArgs, client: &Client) -> Result<()>
where
    K: CustomResourceExt,
{
    let crd = <K as CustomResourceExt>::crd();
    let name = <K as CustomResourceExt>::crd_name();

    let api = Api::all(client.clone());
    if api.get_metadata_opt(name).await?.is_none() {
        let pp = PostParams {
            dry_run: false,
            field_manager: Some(args.controller_name.clone()),
        };
        api.create(&pp, &crd).await?;
        {
            #[cfg(feature = "tracing")]
            info!("created CRD: {name}");
        }
        Ok(())
    } else if args.upgrade_crds {
        let pp = PatchParams {
            dry_run: false,
            force: true,
            field_manager: Some(args.controller_name.clone()),
            field_validation: None,
        };
        api.patch(name, &pp, &Patch::Apply(&crd)).await?;
        {
            #[cfg(feature = "tracing")]
            info!("updated CRD: {name}");
        }
        Ok(())
    } else {
        {
            #[cfg(feature = "tracing")]
            info!("found CRD: {name}");
        }
        Ok(())
    }
}

#[async_trait]
pub trait RecorderExt<R> {
    async fn report_update(
        &self,
        event: &Event,
        reference: &ObjectReference,
    ) -> Result<(), ::kube::Error>;

    async fn report_error(
        &self,
        error: ::kube::runtime::controller::Error<::kube::Error, ::kube::runtime::watcher::Error>,
        reason: R,
        action: String,
    ) where
        R: 'async_trait + Send + ToString;
}

#[async_trait]
impl<R> RecorderExt<R> for Recorder {
    async fn report_update(
        &self,
        event: &Event,
        reference: &ObjectReference,
    ) -> Result<(), ::kube::Error> {
        #[cfg(feature = "tracing")]
        {
            let mut name = String::default();
            if let Some(api_version) = reference.api_version.as_deref() {
                name.push_str(api_version);
                name.push('/');
            }
            if let Some(kind) = reference.kind.as_deref() {
                name.push_str(kind);
                name.push('/');
            }
            if let Some(namespace) = reference.namespace.as_deref() {
                name.push_str(namespace);
                name.push('/');
            }
            name.push_str(reference.name.as_deref().unwrap_or_default());
            info!("{name}: {}", event.note.as_deref().unwrap_or_default());
        }
        self.publish(event, reference).await
    }

    async fn report_error(
        &self,
        error: ::kube::runtime::controller::Error<::kube::Error, ::kube::runtime::watcher::Error>,
        reason: R,
        action: String,
    ) where
        R: 'async_trait + Send + ToString,
    {
        if let ::kube::runtime::controller::Error::ReconcilerFailed(error, object) = &error {
            // Shorten error notes
            let error = match error {
                ::kube::Error::Api(error) => error.to_string(),
                error => error.to_string(),
            };
            let event = Event {
                type_: EventType::Warning,
                reason: reason.to_string(),
                note: Some(error),
                action,
                secondary: None,
            };
            let reference = object.clone().into();
            self.publish(&event, &reference).await.ok();
        }
        #[cfg(feature = "tracing")]
        error!("reconcile failed: {error:?}")
    }
}

/// Return `true` if any condition has been changed.
///
fn is_conditions_changed(a: &[Condition], b: &[Condition]) -> bool {
    a.len() != b.len()
        || a.iter().zip(b.iter()).any(|(a, b)| {
            // Skip validating: last transition time
            a.message != b.message
                || a.observed_generation.is_some() && a.observed_generation != b.observed_generation
                || a.reason != b.reason
                || a.status != b.status
                || a.type_ != b.type_
        })
}

async fn report_update<K, R>(
    recorder: &Recorder,
    object: &K,
    reason: R,
    message: String,
) -> Result<()>
where
    K: Resource,
    <K as Lookup>::DynamicType: Default,
    R: Reason,
{
    let event = ::kube::runtime::events::Event {
        type_: if reason.accepted() {
            ::kube::runtime::events::EventType::Normal
        } else {
            ::kube::runtime::events::EventType::Warning
        },
        reason: reason.to_string(),
        note: Some(message),
        action: "Accepted".into(),
        secondary: None,
    };

    let reference = ObjectRef::from_obj(object).into();
    <Recorder as RecorderExt<R>>::report_update(recorder, &event, &reference).await
}

pub trait Reason
where
    Self: Clone + ToString,
{
    fn accepted(&self) -> bool;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Status<R> {
    pub reason: R,
    pub message: String,
    pub requeue: bool,
}

impl<R> Status<R> {
    fn into_condition(self, metadata: &ObjectMeta) -> Condition
    where
        R: Reason,
    {
        let Self {
            reason,
            message,
            requeue: _,
        } = self;

        Condition {
            last_transition_time: Time(Timestamp::now()),
            message,
            observed_generation: metadata.generation,
            reason: reason.to_string(),
            status: if reason.accepted() {
                "True".into()
            } else {
                "False".into()
            },
            type_: "Accepted".into(),
        }
    }
}

pub trait Resource
where
    Self: Clone + DeserializeOwned + ::kube::Resource<DynamicType = ()> + Lookup<DynamicType = ()>,
{
    type Status: Serialize;

    fn conditions(&self) -> Option<&[Condition]>;

    fn build_status(&self, conditions: Vec<Condition>) -> <Self as Resource>::Status;
}

pub struct Context {
    pub interval: Duration,
    pub patch_params: PatchParams,
    pub recorder: Recorder,
}

impl Context {
    pub async fn commit<K, R>(&self, api: &Api<K>, object: &K, status: Status<R>) -> Result<Action>
    where
        K: Resource,
        R: Reason,
    {
        let Status {
            reason,
            message,
            requeue,
        } = status.clone();

        let metadata = object.meta();
        let conditions = vec![status.into_condition(metadata)];

        // Skip updating status if nothing has been changed
        let last_conditions = object.conditions();
        let has_changed = last_conditions
            .is_none_or(|last_conditions| is_conditions_changed(&conditions, last_conditions));

        if has_changed {
            let name = metadata.name.as_deref().expect("conciled resource");
            let patch = Patch::Merge(json!({
                "apiVersion": <K as ::kube::Resource>::api_version(&()),
                "kind": <K as ::kube::Resource>::kind(&()),
                "status": object.build_status(conditions),
            }));
            api.patch_status(name, &self.patch_params, &patch).await?;
        }

        report_update(&self.recorder, object, reason, message).await?;
        if requeue {
            Ok(Action::requeue(self.interval))
        } else {
            Ok(Action::await_change())
        }
    }
}
