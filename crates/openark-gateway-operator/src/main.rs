use std::sync::Arc;

use anyhow::bail;
use futures::TryStreamExt;
use k8s_openapi::api::core::v1::{ObjectReference, Service};
use kube::{
    Api, Client, Error,
    api::{PatchParams, PostParams, ValidationDirective},
    runtime::{
        controller::Action,
        events::{Recorder, Reporter},
        watcher::{Config, Event, watcher},
    },
};
use openark_core::operator::OperatorArgs;
#[cfg(feature = "tracing")]
use tracing::{Level, error, info, instrument};

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, env = "OPENARK_LABEL_SELECTOR")]
    label_selector: String,

    #[arg(long, env = "NAMESPACE", value_name = "NAME")]
    namespace: Option<String>,

    #[command(flatten)]
    operator: OperatorArgs,
}

#[derive(Clone)]
struct Context {
    api_gateway: Api<Gateway>,
    api_svc: Api<Service>,
    args: Args,
    patch_params: PatchParams,
    post_params: PostParams,
    recorder: Recorder,
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
async fn reconcile(svc: Arc<Service>, ctx: Arc<Context>) -> Result<Action, Error> {
    let name = svc.name_any();

    // Update if changed
    if current != next {
        // Apply patch
        let patch = Patch::Strategic(next.to_patch());
        ctx.api_node.patch(&name, &ctx.patch_params, &patch).await?;
        {
            #[cfg(feature = "tracing")]
            info!("updated node/{name}: {patch:?}");
        }

        // Report message
        let message = if is_app_deleting {
            "Signing out".into()
        } else if must_sign_out {
            "Signed out".into()
        } else if let Some(user) = next.get_user() {
            format!("Signed in with {user}")
        } else {
            "Signed in".into()
        };
        let reference = ObjectRef::from_obj(&*node).into();
        report_update(&ctx.recorder, &reference, message).await?;
    }

    // Completed reconciling
    Ok(Action::await_change())
}

async fn report_update(
    recorder: &Recorder,
    reference: &ObjectReference,
    message: String,
) -> Result<()> {
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
        info!("{name}: {message}");
    }

    let event = ::kube::runtime::events::Event {
        type_: ::kube::runtime::events::EventType::Normal,
        reason: "Reloaded".into(),
        note: Some(message),
        action: "Provisioning".into(),
        secondary: None,
    };
    recorder.publish(&event, reference).await
}

async fn report_error(
    recorder: &Recorder,
    error: ::kube::runtime::controller::Error<Error, ::kube::runtime::watcher::Error>,
) {
    if let ::kube::runtime::controller::Error::ReconcilerFailed(error, object) = &error {
        // Shorten error notes
        let error = match error {
            Error::Api(error) => error.to_string(),
            error => error.to_string(),
        };
        let event = ::kube::runtime::events::Event {
            type_: ::kube::runtime::events::EventType::Warning,
            reason: "ReloadError".into(),
            note: Some(error),
            action: "Provisioning".into(),
            secondary: None,
        };
        let reference = object.clone().into();
        recorder.publish(&event, &reference).await.ok();
    }
    #[cfg(feature = "tracing")]
    error!("reconcile failed: {error:?}")
}

async fn try_main(args: Args) -> Result<()> {
    let client = Client::try_default().await?;

    let api_gateway = Api::default_namespaced(client.clone());
    let api_svc = match args.namespace.as_deref() {
        Some(ns) => Api::namespaced(client.clone(), ns),
        None => Api::all(client.clone()),
    };

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
    let recorder = Recorder::new(client, reporter);

    let watcher_config = Config {
        label_selector: Some(args.label_selector.clone()),
        ..Default::default()
    };

    let context = Arc::new(Context {
        api_gateway,
        api_svc: api_svc.clone(),
        args,
        patch_params,
        post_params,
        recorder: recorder.clone(),
    });

    let mut stream = Box::pin(watcher(api_svc, watcher_config));
    while let Some(event) = stream.try_next().await? {
        match event {
            Event::Apply(svc) | Event::InitApply(svc) => service.apply(&node).await?,
            Event::Delete(svc) => {
                {
                    #[cfg(feature = "tracing")]
                    warn!("deleting node: {}", &args.node_name);
                }
                service.stop().await?
            }
            Event::Init | Event::InitDone => continue,
        }
    }
    bail!("unexpected termination")
}

#[::tokio::main]
async fn main() {
    let args = Args::parse();

    ::openark_core::init_once();

    #[cfg(feature = "tracing")]
    ::tracing::info!("Welcome to OpenARK VINE Ollama Operator!");

    try_main(args).await.expect("running an operator")
}
