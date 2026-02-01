use std::{sync::Arc, time::Duration};

use anyhow::Result;
use futures::StreamExt;
use jiff::Timestamp;
use k8s_openapi::api::core::v1::ObjectReference;
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
use openark_kiss_ansible::{AnsibleClient, AnsibleJob, AnsibleResourceType};
use openark_kiss_api::r#box::{BoxCrd, BoxGroupRole, BoxGroupSpec, BoxSpec, BoxState, BoxStatus};
use serde_json::json;
#[cfg(feature = "tracing")]
use tracing::{Level, info, instrument};

use crate::status::Reason;

struct Context {
    ansible: AnsibleClient,
    api: Api<BoxCrd>,
    crd: ApiResource,
    enable_cronjobs: bool,
    interval: Duration,
    patch_params: PatchParams,
    recorder: Recorder,
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
async fn reconcile(r#box: Arc<BoxCrd>, ctx: Arc<Context>) -> Result<Action, Error> {
    let reference = ObjectRef::from_obj(&*r#box).into();
    let name = r#box.name_any();
    let status = r#box.status.as_ref();

    // get the current time
    let now = Timestamp::now();

    // load the box's state
    let old_state = status
        .as_ref()
        .map(|status| status.state)
        .unwrap_or(BoxState::New);
    let mut new_state = old_state.next();
    let mut new_group = None;

    // apply the default role
    let default_role = ctx.ansible.kiss.group_default_role;
    if matches!(r#box.spec.group.role, BoxGroupRole::GenericWorker)
        && r#box.spec.group.role != default_role
    {
        let patch = Patch::Merge(json!({
            "apiVersion": &ctx.crd.api_version,
            "kind": &ctx.crd.kind,
            "spec": BoxSpec {
                group: BoxGroupSpec {
                    role: default_role,
                    ..r#box.spec.group.clone()
                },
                ..r#box.spec.clone()
            },
        }));
        ctx.api.patch(&name, &ctx.patch_params, &patch).await?;

        let message = format!("Changed the default role to {default_role}");
        report_update(&ctx.recorder, &reference, message).await?;
        return Ok(Action::requeue(ctx.interval));
    }

    // detect the box's group is changed
    let is_bind_group_updated = status
        .as_ref()
        .and_then(|status| status.bind_group.as_ref())
        .map(|bind_group| bind_group != &r#box.spec.group)
        .unwrap_or(true);

    // wait new boxes with no access methods for begin provisioned
    if matches!(old_state, BoxState::New)
        && !status
            .as_ref()
            .map(|status| status.access.primary.is_some())
            .unwrap_or_default()
    {
        let timeout = BoxState::timeout_new();

        if let Some(last_updated) = r#box.last_updated() {
            if now > last_updated + timeout {
                // update the status
                new_state = BoxState::Disconnected;
            } else {
                return Ok(Action::requeue(timeout.unsigned_abs()));
            }
        } else {
            return Ok(Action::requeue(timeout.unsigned_abs()));
        }
    }

    // capture the timeout
    if let Some(time_threshold) = old_state.timeout()
        && let Some(last_updated) = r#box.last_updated()
        && now > last_updated + time_threshold
    {
        // update the status
        new_state = BoxState::Failed;
    }

    // capture the group info is changed
    if matches!(old_state, BoxState::Running) && is_bind_group_updated {
        new_state = BoxState::Disconnected;
    }

    if !matches!(old_state, BoxState::Joining) && matches!(new_state, BoxState::Joining) {
        // skip joining to default cluster as worker nodes when external
        if matches!(r#box.spec.group.role, BoxGroupRole::ExternalWorker) {
            let message = "Skipped joining (box is external)".into();
            report_update(&ctx.recorder, &reference, message).await?;
            return Ok(Action::requeue(ctx.interval));
        }

        // skip joining to default cluster as worker nodes when disabled
        if !ctx.ansible.kiss.group_enable_default_cluster
            && r#box.spec.group.is_default()
            && matches!(r#box.spec.group.role, BoxGroupRole::GenericWorker)
        {
            let message = "Skipped joining (default cluster is disabled)".into();
            report_update(&ctx.recorder, &reference, message).await?;
            return Ok(Action::requeue(ctx.interval));
        }

        // skip joining if already joined
        if !is_bind_group_updated {
            let patch = Patch::Merge(json!({
                "apiVersion": &ctx.crd.api_version,
                "kind": &ctx.crd.kind,
                "status": BoxStatus {
                    access: status.map(|status| status.access.clone()).unwrap_or_default(),
                    state: BoxState::Running,
                    bind_group: status.and_then(|status| status.bind_group.clone()),
                    last_updated: Timestamp::now(),
                },
            }));
            ctx.api
                .patch_status(&name, &ctx.patch_params, &patch)
                .await?;

            let message = "Skipped joining (already joined)".into();
            report_update(&ctx.recorder, &reference, message).await?;
            return Ok(Action::requeue(ctx.interval));
        }

        // bind to new group
        new_group = Some(&r#box.spec.group);
    }

    // spawn an Ansible job
    if old_state != new_state || ctx.enable_cronjobs && new_state.cron().is_some() {
        if let Some(task) = new_state.as_task() {
            let is_spawned = ctx
                .ansible
                .spawn(AnsibleJob {
                    cron: new_state.cron(),
                    task,
                    r#box: &r#box,
                    new_group,
                    new_state: Some(new_state),
                    is_critical: false,
                    resource_type: match old_state {
                        BoxState::New
                        | BoxState::Commissioning
                        | BoxState::Ready
                        | BoxState::Joining => AnsibleResourceType::Normal,
                        BoxState::Running
                        | BoxState::GroupChanged
                        | BoxState::Failed
                        | BoxState::Disconnected => AnsibleResourceType::Minimal,
                    },
                    use_workers: false,
                })
                .await?;

            // If there is a problem spawning a job, check back after a few minutes
            if !is_spawned {
                let message = "Cannot spawn an Ansible job; waiting".into();
                report_update(&ctx.recorder, &reference, message).await?;
                return Ok(Action::requeue(
                    #[allow(clippy::identity_op)]
                    Duration::from_secs(1 * 60),
                ));
            }
        }

        // wait for being changed
        if old_state == new_state {
            let message = "Waiting for being changed".into();
            report_update(&ctx.recorder, &reference, message).await?;
            return Ok(Action::await_change());
        }

        // bind group before joining to a cluster
        let bind_group = if matches!(new_state, BoxState::Joining) {
            Some(&r#box.spec.group)
        } else {
            status
                .as_ref()
                .and_then(|status| status.bind_group.as_ref())
        };

        let patch = Patch::Merge(json!({
            "apiVersion": &ctx.crd.api_version,
            "kind": &ctx.crd.kind,
            "status": BoxStatus {
                access: status.map(|status| status.access.clone()).unwrap_or_default(),
                state: new_state,
                bind_group: bind_group.cloned(),
                last_updated: Timestamp::now(),
            },
        }));
        ctx.api
            .patch_status(&name, &ctx.patch_params, &patch)
            .await?;

        let message = format!("Updated state: {new_state}");
        report_update(&ctx.recorder, &reference, message).await?;
    }

    if old_state == new_state {
        let message = "Waiting for being changed".into();
        report_update(&ctx.recorder, &reference, message).await?;
        Ok(Action::await_change())
    } else {
        // If no events were received, check back after a few seconds
        Ok(Action::requeue(ctx.interval))
    }
}

async fn report_update(
    recorder: &Recorder,
    reference: &ObjectReference,
    message: String,
) -> Result<(), ::kube::Error> {
    let event = ::kube::runtime::events::Event {
        type_: ::kube::runtime::events::EventType::Normal,
        reason: Reason::ProvisioningUpdated.to_string(),
        note: Some(message),
        action: "Scheduling".into(),
        secondary: None,
    };
    RecorderExt::<Reason>::report_update(recorder, &event, reference).await
}

async fn report_error(
    recorder: &Recorder,
    error: ::kube::runtime::controller::Error<Error, ::kube::runtime::watcher::Error>,
) {
    let reason = Reason::ProvisioningError;
    let action = "Scheduling".into();
    recorder.report_error(error, reason, action).await
}

fn error_policy(_box: Arc<BoxCrd>, _error: &Error, ctx: Arc<Context>) -> Action {
    Action::requeue(ctx.interval)
}

pub async fn loop_forever(args: super::Args, client: Client) -> Result<()> {
    let namespace = args
        .operator
        .namespace
        .as_deref()
        .unwrap_or(client.default_namespace());
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
        ansible: AnsibleClient::try_new(&client, namespace).await?,
        api: api.clone(),
        crd: BoxCrd::api_resource(),
        enable_cronjobs: args.enable_cronjobs,
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
