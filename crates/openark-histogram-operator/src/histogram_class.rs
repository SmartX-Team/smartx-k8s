use std::{sync::Arc, time::Duration};

use anyhow::Result;
use chrono::Utc;
use futures::StreamExt;
use k8s_openapi::{
    Resource,
    api::core::v1::Service,
    apimachinery::pkg::apis::meta::v1::{Condition, Time},
};
use kube::{
    Api, Client, CustomResourceExt, Error, ResourceExt,
    api::{ApiResource, Patch, PatchParams, ValidationDirective},
    runtime::{
        Controller,
        controller::Action,
        events::{Recorder, Reporter},
        reflector::ObjectRef,
        watcher::Config,
    },
};
use openark_core::{
    client::{Client as _, HealthState},
    operator::{RecorderExt, is_conditions_changed},
};
use openark_histogram_api::{
    common::{ObjectReference, ServiceReference},
    histogram_class::{HistogramClassCrd, HistogramClassStatus},
};
use serde_json::json;
#[cfg(feature = "tracing")]
use tracing::{Level, info, instrument};

use crate::utils::build_service_reference_url_by_service;

struct ValidationError {
    message: String,
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
async fn validate_service(
    client: &::reqwest::Client,
    svc: &Service,
    port: u16,
) -> Result<Result<(), ValidationError>, Error> {
    // Validate port
    if port < 1024 && port != 80 && port != 443 || port >= 40000 {
        return Ok(Err(ValidationError {
            message: "Invalid port number".into(),
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
        return Ok(Err(ValidationError {
            message: "Undefined port number".into(),
        }));
    }

    // Validate service API
    let url = match build_service_reference_url_by_service(svc, port) {
        Ok(url) => url,
        Err(error) => {
            return Ok(Err(ValidationError {
                message: error.to_string(),
            }));
        }
    };
    match client.health(url.clone()).await {
        Ok(HealthState::Healthy) => Ok(Ok(())),
        Err(error) => {
            return Ok(Err(ValidationError {
                message: format!("Unhealthy service: {error}: {url}"),
            }));
        }
    }
}

struct Context {
    api: Api<HistogramClassCrd>,
    client: ::reqwest::Client,
    controller_name: String,
    crd: ApiResource,
    interval: Duration,
    kube: Client,
    patch_params: PatchParams,
    recorder: Recorder,
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
async fn reconcile(class: Arc<HistogramClassCrd>, ctx: Arc<Context>) -> Result<Action, Error> {
    let name = class.name_any();

    // Skip if the controller name has mismatched
    if class.spec.controller_name != ctx.controller_name {
        return Ok(Action::await_change());
    }

    // Define status update function
    let update_status = |reason: String, message: String| async {
        let status = HistogramClassStatus {
            conditions: vec![Condition {
                last_transition_time: Time(Utc::now()),
                message: message.clone(),
                observed_generation: class.metadata.generation,
                reason: reason.clone(),
                status: match reason.as_str() {
                    "Accepted" => "True".into(),
                    _ => "False".into(),
                },
                type_: "Accepted".into(),
            }],
        };

        // Skip updating status if nothing has been changed
        let has_changed =
            class
                .status
                .as_ref()
                .is_none_or(|HistogramClassStatus { conditions }| {
                    is_conditions_changed(conditions, &status.conditions)
                });

        if has_changed {
            let patch = Patch::Merge(json!({
                "apiVersion": ctx.crd.api_version,
                "kind": ctx.crd.kind,
                "status": status,
            }));
            ctx.api
                .patch_status(&name, &ctx.patch_params, &patch)
                .await?;
        }

        report_update(&ctx.recorder, &class, reason, message).await
    };

    // Define accept function
    let update_status_accepted = || async {
        let reason = "Accepted".into();
        let message = "Valid HistogramClass".into();
        update_status(reason, message).await?;
        Ok(Action::await_change())
    };

    // Define invalid backend error function
    let update_status_invalid_backend_ref = |message, action| async {
        let reason = "InvalidBackendRef".into();
        update_status(reason, message).await?;
        Ok(action)
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
    } = &class.spec.backend_ref;

    match (group.as_str(), kind.as_str(), namespace.as_deref()) {
        (Service::GROUP, Service::KIND, Some(namespace)) => {
            let api = Api::<Service>::namespaced(ctx.kube.clone(), namespace);
            match api.get_opt(name).await? {
                Some(svc) => match validate_service(&ctx.client, &svc, *port).await? {
                    Ok(()) => update_status_accepted().await,
                    Err(ValidationError { message }) => {
                        let action = Action::requeue(ctx.interval);
                        update_status_invalid_backend_ref(message, action).await
                    }
                },
                None => {
                    let message = format!("Missing backend: {kind}/{namespace}/{name}");
                    let action = Action::requeue(ctx.interval);
                    update_status_invalid_backend_ref(message, action).await
                }
            }
        }
        (Service::GROUP, Service::KIND, None) => {
            let message = format!("Required backend namespace: {kind}/???/{name}");
            let action = Action::await_change();
            update_status_invalid_backend_ref(message, action).await
        }
        (group, kind, _) => {
            let message = if group.is_empty() {
                format!("Unsupported backend kind: {kind}")
            } else {
                format!("Unsupported backend kind: {group}/{kind}")
            };
            let action = Action::await_change();
            update_status_invalid_backend_ref(message, action).await
        }
    }
}

async fn report_update(
    recorder: &Recorder,
    class: &HistogramClassCrd,
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
    let reference = ObjectRef::from_obj(class).into();
    recorder.report_update(&event, &reference).await
}

async fn report_error(
    recorder: &Recorder,
    error: ::kube::runtime::controller::Error<Error, ::kube::runtime::watcher::Error>,
) {
    let reason = "InvalidBackendRef".into();
    let action = "Accepted".into();
    recorder.report_error(error, reason, action).await
}

fn error_policy(_class: Arc<HistogramClassCrd>, _error: &Error, ctx: Arc<Context>) -> Action {
    Action::requeue(ctx.interval)
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
        crd: HistogramClassCrd::api_resource(),
        interval: Duration::from_secs(30),
        kube,
        patch_params,
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
