use std::{sync::Arc, time::Duration};

use anyhow::Result;
use futures::StreamExt;
use k8s_openapi::{Resource, api::core::v1::Service};
use kube::{
    Api, Client, Error,
    api::{PatchParams, ValidationDirective},
    runtime::{
        Controller,
        controller::Action,
        events::{Recorder, Reporter},
        watcher::Config,
    },
};
use openark_core::{
    client::{Client as _, HealthState},
    operator::RecorderExt,
};
use openark_spectrum_api::{
    common::{ObjectReference, ServiceReference},
    spectrum_class::{SpectrumClassCrd, SpectrumClassSpec},
};
#[cfg(feature = "tracing")]
use tracing::{Level, info, instrument};

use crate::{
    status::{Reason, Status},
    utils::build_service_reference_url_by_service,
};

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
async fn validate_service(
    client: &::reqwest::Client,
    svc: &Service,
    port: u16,
) -> Result<Result<(), Status>, Error> {
    // Validate port
    if port < 1024 && port != 80 && port != 443 || port >= 40000 {
        return Ok(Err(Status {
            reason: Reason::InvalidBackendRef,
            message: "Invalid port number".into(),
            requeue: false,
        }));
    }
    let service_port: i32 = port as _;

    // Validate service port
    if svc
        .spec
        .as_ref()
        .and_then(|spec| spec.ports.as_ref())
        .is_none_or(|ports| {
            ports
                .iter()
                .filter(|&p| p.protocol.as_ref().is_none_or(|protocol| protocol == "TCP"))
                .all(|p| p.port != service_port)
        })
    {
        return Ok(Err(Status {
            reason: Reason::InvalidBackendRef,
            message: "Undefined port number".into(),
            requeue: true,
        }));
    }

    // Validate service API
    let url = match build_service_reference_url_by_service(svc, port) {
        Ok(url) => url,
        Err(error) => {
            return Ok(Err(Status {
                reason: Reason::InvalidBackendRef,
                message: error.to_string(),
                requeue: true,
            }));
        }
    };
    match client.health(url.clone()).await {
        Ok(HealthState::Healthy) => Ok(Ok(())),
        Err(error) => {
            return Ok(Err(Status {
                reason: Reason::InvalidBackendRef,
                message: format!("Unhealthy service: {error}: {url}"),
                requeue: true,
            }));
        }
    }
}

struct Context {
    api: Api<SpectrumClassCrd>,
    client: ::reqwest::Client,
    controller_name: String,
    kube: Client,
    status: ::openark_core::operator::Context,
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
async fn reconcile(class: Arc<SpectrumClassCrd>, ctx: Arc<Context>) -> Result<Action, Error> {
    let SpectrumClassSpec {
        controller_name,
        description: _,
        backend_ref,
    } = &class.spec;

    // Skip if the controller name has mismatched
    if *controller_name != ctx.controller_name {
        return Ok(Action::await_change());
    }

    // Define status update function
    let commit = |status: Status| {
        let api = &ctx.api;
        let object = &*class;
        ctx.status.commit(api, object, status)
    };

    // Define accept function
    let commit_ok = || {
        commit(Status {
            reason: Reason::Accepted,
            message: "Valid SpectrumClass".into(),
            requeue: false,
        })
    };

    // Define invalid backend error function
    let commit_invalid_backend_ref = |message, requeue| {
        commit(Status {
            reason: Reason::InvalidBackendRef,
            message,
            requeue,
        })
    };

    // Validate backend service
    let ServiceReference {
        object:
            ObjectReference {
                group,
                kind,
                name,
                namespace,
            },
        port,
    } = backend_ref;

    match (group.as_str(), kind.as_str(), namespace.as_deref()) {
        (Service::GROUP, Service::KIND, Some(namespace)) => {
            let api = Api::<Service>::namespaced(ctx.kube.clone(), namespace);
            match api.get_opt(name).await? {
                Some(svc) => match validate_service(&ctx.client, &svc, *port).await? {
                    Ok(()) => commit_ok().await,
                    Err(status) => commit(status).await,
                },
                None => {
                    let message = format!("Missing backend: {kind}/{namespace}/{name}");
                    let requeue = true;
                    commit_invalid_backend_ref(message, requeue).await
                }
            }
        }
        (Service::GROUP, Service::KIND, None) => {
            let message = format!("Required backend namespace: {kind}/???/{name}");
            let requeue = false;
            commit_invalid_backend_ref(message, requeue).await
        }
        (group, kind, _) => {
            let message = if group.is_empty() {
                format!("Unsupported backend kind: {kind}")
            } else {
                format!("Unsupported backend kind: {group}/{kind}")
            };
            let requeue = false;
            commit_invalid_backend_ref(message, requeue).await
        }
    }
}

async fn report_error(
    recorder: &Recorder,
    error: ::kube::runtime::controller::Error<Error, ::kube::runtime::watcher::Error>,
) {
    let reason = Reason::ProvisioningError;
    let action = "Accepted".into();
    recorder.report_error(error, reason, action).await
}

fn error_policy(_class: Arc<SpectrumClassCrd>, _error: &Error, ctx: Arc<Context>) -> Action {
    Action::requeue(ctx.status.interval)
}

pub async fn loop_forever(
    args: super::Args,
    client: ::reqwest::Client,
    kube: Client,
) -> Result<()> {
    let api = Api::all(kube.clone());

    let patch_params = PatchParams {
        dry_run: false,
        force: false,
        field_manager: Some(args.operator.controller_name.clone()),
        field_validation: Some(ValidationDirective::Strict),
    };

    let reporter = Reporter {
        controller: args.operator.controller_name.clone(),
        instance: args.operator.controller_pod_name.clone(),
    };
    let recorder = Recorder::new(kube.clone(), reporter);

    let watcher_config = Config::default();

    let context = Arc::new(Context {
        api: api.clone(),
        client,
        controller_name: args.operator.controller_name.clone(),
        kube,
        status: ::openark_core::operator::Context {
            interval: Duration::from_secs(30),
            patch_params,
            recorder: recorder.clone(),
        },
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
