use std::{sync::Arc, time::Duration};

use anyhow::Result;
use chrono::Utc;
use futures::StreamExt;
use k8s_openapi::api::{batch::v1::Job, core::v1::ObjectReference};
use kube::{
    Api, Client, CustomResourceExt, Error, ResourceExt,
    api::{ApiResource, Patch, PatchParams, ValidationDirective},
    runtime::{
        Controller,
        controller::Action,
        events::{Event, EventType, Recorder, Reporter},
        watcher::Config,
    },
};
use openark_kiss_ansible::AnsibleClient;
use openark_kiss_api::r#box::{BoxCrd, BoxState};
use serde_json::json;
#[cfg(feature = "tracing")]
use tracing::{Level, error, info, instrument, warn};

struct Context {
    api_box: Api<BoxCrd>,
    crd_box: ApiResource,
    interval: Duration,
    patch_params: PatchParams,
    recorder: Recorder,
}

#[cfg_attr(feature = "tracing", instrument(
    level = Level::INFO,
    skip_all,
    fields(name = %job.name_any(), namespace = job.namespace(), state = %state),
    err(Display),
))]
async fn update_box_state(
    job: &Job,
    ctx: &Context,
    reference: &ObjectReference,
    state: BoxState,
) -> Result<Action, Error> {
    // box name is already tested by reconciling
    let box_name = get_box_name(job).unwrap();

    // update the box
    {
        let patch = Patch::Strategic(json!({
            "apiVersion": &ctx.crd_box.api_version,
            "kind": &ctx.crd_box.kind,
            "metadata": {
                "name": &box_name,
            },
            "status": {
                "state": state,
                "lastUpdated": Utc::now(),
            },
        }));
        ctx.api_box
            .patch_status(&box_name, &ctx.patch_params, &patch)
            .await?;
    }

    {
        let message = format!("Updated state: {state}");
        report_update(&ctx.recorder, reference, message).await?;
    }
    Ok(Action::requeue(ctx.interval))
}

fn get_box_name(job: &Job) -> Option<String> {
    get_label(job, AnsibleClient::LABEL_BOX_NAME)
}

fn get_label<T>(job: &Job, label: &str) -> Option<T>
where
    T: ::core::str::FromStr + Send,
    <T as ::core::str::FromStr>::Err: ::core::fmt::Display + Send,
{
    match job.labels().get(label) {
        Some(value) => match value.parse() {
            Ok(value) => Some(value),
            Err(error) => {
                #[cfg(feature = "tracing")]
                warn!(
                    "failed to parse the {label} label of {}: {error}",
                    job.name_any(),
                );
                None
            }
        },
        None => {
            #[cfg(feature = "tracing")]
            info!("failed to get the {label} label: {}", job.name_any());
            None
        }
    }
}

fn is_critical(job: &Job) -> bool {
    get_label(job, AnsibleClient::LABEL_JOB_IS_CRITICAL).unwrap_or_default()
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
async fn reconcile(job: Arc<Job>, ctx: Arc<Context>) -> Result<Action, Error> {
    let name = job.name_any();

    // skip reconciling if not managed
    let box_name = match get_box_name(&job) {
        Some(name) => name,
        None => {
            #[cfg(feature = "tracing")]
            info!("{name} is not a target; skipping");
            return Ok(Action::await_change());
        }
    };
    let box_reference = ObjectReference {
        api_version: Some(ctx.crd_box.api_version.clone()),
        field_path: None,
        kind: Some(ctx.crd_box.kind.clone()),
        name: Some(box_name.clone()),
        namespace: None,
        resource_version: None,
        uid: None,
    };

    // skip reconciling if critical
    if is_critical(&job) {
        #[cfg(feature = "tracing")]
        info!("{name} is a critical job; skipping");
        return Ok(Action::await_change());
    }

    let status = job.status.as_ref();
    let completed_state = job
        .labels()
        .get(AnsibleClient::LABEL_COMPLETED_STATE)
        .and_then(|state| state.parse().ok());

    let has_completed = status.and_then(|e| e.succeeded).unwrap_or_default() > 0;
    let has_failed = status.and_then(|e| e.failed).unwrap_or_default() > 0;

    // when the ansible job is succeeded
    if has_completed {
        #[cfg(feature = "tracing")]
        info!("Job has completed: {name} ({box_name})");

        // update the state
        if let Some(completed_state) = completed_state {
            #[cfg(feature = "tracing")]
            info!("Updating box state: {name} ({box_name} => {completed_state})");
            update_box_state(&job, &ctx, &box_reference, completed_state).await
        }
        // keep the state, scheduled by the controller
        else {
            #[cfg(feature = "tracing")]
            info!("Skipping updating box state: {name} ({box_name})");
            Ok(Action::requeue(ctx.interval))
        }
    }
    // when the ansible job is failed
    else if has_failed {
        let failed_state = BoxState::Failed;
        #[cfg(feature = "tracing")]
        {
            warn!("Job has failed: {name} ({box_name})");
            warn!("Updating box state: {name} ({box_name} => {failed_state})");
        }

        update_box_state(&job, &ctx, &box_reference, failed_state).await
    }
    // when the ansible job is not finished yet
    else {
        Ok(Action::requeue(ctx.interval))
    }
}

async fn report_update(
    recorder: &Recorder,
    reference: &ObjectReference,
    message: String,
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
        info!("{name}: {message}");
    }

    let event = Event {
        type_: EventType::Normal,
        reason: "ProvisioningUpdated".into(),
        note: Some(message),
        action: "Scheduling".into(),
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
        let event = Event {
            type_: EventType::Warning,
            reason: "ProvisioningError".into(),
            note: Some(error),
            action: "Scheduling".into(),
            secondary: None,
        };
        let reference = object.clone().into();
        recorder.publish(&event, &reference).await.ok();
    }
    #[cfg(feature = "tracing")]
    error!("reconcile failed: {error:?}")
}

fn error_policy(_box: Arc<Job>, _error: &Error, _ctx: Arc<Context>) -> Action {
    Action::requeue(Duration::from_secs(30))
}

pub async fn loop_forever(args: super::Args, client: Client) -> Result<()> {
    let api = Api::<Job>::namespaced(client.clone(), &args.namespace);

    let patch_params = PatchParams {
        dry_run: false,
        force: false,
        field_manager: Some(args.operator.controller_name.clone()),
        field_validation: Some(ValidationDirective::Strict),
    };

    let reporter = Reporter {
        controller: args.operator.controller_name.clone(),
        instance: args.controller_pod_name.clone(),
    };
    let recorder = Recorder::new(client.clone(), reporter);

    let watcher_config = Config::default();

    let context = Arc::new(Context {
        api_box: Api::all(client),
        crd_box: BoxCrd::api_resource(),
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
