use async_trait::async_trait;
#[cfg(feature = "clap")]
use clap::Parser;
use k8s_openapi::api::core::v1::ObjectReference;
use kube::{
    Api, Client, CustomResourceExt, Result,
    api::{Patch, PatchParams, PostParams},
    runtime::events::{Event, EventType, Recorder},
};
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
pub trait RecorderExt {
    async fn report_update(
        &self,
        event: &Event,
        reference: &ObjectReference,
    ) -> Result<(), ::kube::Error>;

    async fn report_error(
        &self,
        error: ::kube::runtime::controller::Error<::kube::Error, ::kube::runtime::watcher::Error>,
        reason: String,
        action: String,
    );
}

#[async_trait]
impl RecorderExt for Recorder {
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
        reason: String,
        action: String,
    ) {
        if let ::kube::runtime::controller::Error::ReconcilerFailed(error, object) = &error {
            // Shorten error notes
            let error = match error {
                ::kube::Error::Api(error) => error.to_string(),
                error => error.to_string(),
            };
            let event = Event {
                type_: EventType::Warning,
                reason,
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
