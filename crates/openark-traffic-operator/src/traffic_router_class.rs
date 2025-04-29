use std::{sync::Arc, time::Duration};

use anyhow::Result;
use chrono::Utc;
use futures::StreamExt;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{Condition, Time};
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
use openark_core::operator::RecorderExt;
use openark_traffic_api::traffic_router_class::{TrafficRouterClassCrd, TrafficRouterClassStatus};
use serde_json::json;
#[cfg(feature = "tracing")]
use tracing::{Level, info, instrument};

struct Context {
    api: Api<TrafficRouterClassCrd>,
    controller_name: String,
    crd: ApiResource,
    interval: Duration,
    patch_params: PatchParams,
    recorder: Recorder,
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
async fn reconcile(class: Arc<TrafficRouterClassCrd>, ctx: Arc<Context>) -> Result<Action, Error> {
    let name = class.name_any();

    // Skip if the controller name has mismatched
    if class.spec.controller_name != ctx.controller_name {
        return Ok(Action::await_change());
    }

    // Define status update function
    let update_status = |reason: String, message: String| async {
        let patch = Patch::Merge(json!({
            "apiVersion": ctx.crd.api_version,
            "kind": ctx.crd.kind,
            "status": TrafficRouterClassStatus {
                conditions: vec![
                    Condition {
                        last_transition_time: Time(Utc::now()),
                        message: message.clone(),
                        observed_generation: class.metadata.generation,
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
        ctx.api
            .patch_status(&name, &ctx.patch_params, &patch)
            .await?;

        report_update(&ctx.recorder, &class, reason, message).await
    };

    // Validate parameters
    if let Some(parameters_ref) = class.spec.parameters_ref.as_ref() {
        // TODO: Add support for parametersRef
        let reason = "InvalidParameters".into();
        let message = "parametersRef is not supported yet".into();
        update_status(reason, message).await?;
        return Ok(Action::await_change());
    }

    {
        let reason = "Accepted".into();
        let message = "Valid TrafficRouterClass".into();
        update_status(reason, message).await?;
        Ok(Action::await_change())
    }
}

async fn report_update(
    recorder: &Recorder,
    class: &TrafficRouterClassCrd,
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
    let reason = "InvalidParameters".into();
    let action = "Accepted".into();
    recorder.report_error(error, reason, action).await
}

fn error_policy(_class: Arc<TrafficRouterClassCrd>, _error: &Error, ctx: Arc<Context>) -> Action {
    Action::requeue(ctx.interval)
}

pub async fn loop_forever(args: super::Args, client: Client) -> Result<()> {
    let api = Api::all(client.clone());

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
    let recorder = Recorder::new(client.clone(), reporter);

    let watcher_config = Config::default();

    let context = Arc::new(Context {
        api: api.clone(),
        controller_name: args.operator.controller_name.clone(),
        crd: TrafficRouterClassCrd::api_resource(),
        interval: Duration::from_secs(30),
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
