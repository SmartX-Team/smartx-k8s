mod service;

use std::{sync::Arc, time::Duration};

use anyhow::Result;
use futures::StreamExt;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference;
use kube::{
    Api, Client, Error, Resource, ResourceExt,
    api::{ObjectMeta, PatchParams, PostParams, ValidationDirective},
    runtime::{
        Controller,
        controller::Action,
        events::{Recorder, Reporter},
        watcher::Config,
    },
};
use openark_core::operator::RecorderExt;
use openark_spectrum_api::{
    pool::{PoolCrd, PoolSpec},
    pool_claim::{PoolClaimCrd, PoolClaimSpec, PoolResourceLifecycle, PoolResourceSettings},
};
#[cfg(feature = "tracing")]
use tracing::{Level, info, instrument};

use crate::{
    status::{Reason, Status},
    targets::{Target, get_target},
    utils::get_pool,
};

struct Context {
    kube: Client,
    label_lifecycle_pre_start: String,
    label_parent: String,
    label_pool_parent: String,
    label_priority: String,
    label_weight: String,
    label_weight_penalty: String,
    label_weight_max: String,
    label_weight_min: String,
    post_params: PostParams,
    status: ::openark_core::operator::Context,
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
async fn reconcile(claim: Arc<PoolClaimCrd>, ctx: Arc<Context>) -> Result<Action, Error> {
    let metadata = &claim.metadata;
    let name = claim.name_any();
    let namespace = metadata.namespace.as_deref().expect("Namespaced resource");
    let PoolClaimSpec {
        pool_name,
        lifecycle: PoolResourceLifecycle { pre_start },
        resources:
            PoolResourceSettings {
                penalty,
                priority,
                weight,
                max,
                min,
            },
    } = &claim.spec;

    // Define status update function
    let api = Api::namespaced(ctx.kube.clone(), namespace);
    let commit = |status: Status| {
        let api = &api;
        let object = &*claim;
        ctx.status.commit(api, object, status)
    };

    // Define accept function
    let commit_ok = || {
        commit(Status {
            reason: Reason::Accepted,
            message: "Valid PoolClaim".into(),
            requeue: false,
        })
    };

    // Validate pool
    let api_pool = Api::namespaced(ctx.kube.clone(), namespace);
    let PoolCrd {
        metadata: _,
        spec: PoolSpec {
            spectrum_class_name: _,
            target_ref,
        },
        status: _,
    } = match get_pool(&api_pool, pool_name).await? {
        Ok(pool) => pool,
        Err(action) => return commit(action).await,
    };

    // Validate target
    let (target_metadata, target_spec) = match get_target(&ctx.kube, namespace, &target_ref).await?
    {
        Ok(list) => list,
        Err(action) => return commit(action).await,
    };

    // Build owner references
    let child_owner_references = vec![OwnerReference {
        block_owner_deletion: Some(true),
        controller: Some(false),
        ..claim
            .owner_ref(&Default::default())
            .expect("pool claim owner reference")
    }];

    // Build metadata
    let child_metadata = ObjectMeta {
        name: Some(name.clone()),
        namespace: Some(namespace.into()),
        annotations: {
            let mut map = target_metadata.annotations.clone().unwrap_or_default();
            if let Some(mut claim_map) = metadata.annotations.clone() {
                map.append(&mut claim_map);
            }
            // resources
            {
                map.insert(
                    ctx.label_weight_penalty.clone(),
                    penalty.map(|x| x.to_string()).unwrap_or_default(),
                );
                map.insert(
                    ctx.label_weight_max.clone(),
                    max.map(|x| x.to_string()).unwrap_or_default(),
                );
                map.insert(
                    ctx.label_weight_min.clone(),
                    min.map(|x| x.to_string()).unwrap_or_default(),
                );
            }
            Some(map)
        },
        labels: {
            let mut map = target_metadata.labels.clone().unwrap_or_default();
            if let Some(mut claim_map) = metadata.labels.clone() {
                map.append(&mut claim_map);
            }
            map.insert(ctx.label_parent.clone(), name.clone());
            map.insert(ctx.label_pool_parent.clone(), pool_name.clone());
            // lifecycle
            {
                map.insert(
                    ctx.label_lifecycle_pre_start.clone(),
                    pre_start.len().to_string(),
                );
            }
            // resources
            {
                map.insert(
                    ctx.label_priority.clone(),
                    priority.unwrap_or(0).to_string(),
                );
                map.insert(ctx.label_weight.clone(), weight.unwrap_or(1).to_string());
            }
            Some(map)
        },
        owner_references: Some(child_owner_references),
        ..Default::default()
    };

    // Create/Update a target resource
    match match target_spec {
        Target::Service(target_spec) => {
            let child_ctx = self::service::Context {
                child_metadata,
                kube: &ctx.kube,
                name: &name,
                namespace,
                post_params: &ctx.post_params,
                target_spec,
            };
            self::service::update_resources(child_ctx).await?
        }
    } {
        // Completed provisioning
        Ok(()) => commit_ok().await,
        Err(action) => commit(action).await,
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

fn error_policy(_claim: Arc<PoolClaimCrd>, _error: &Error, ctx: Arc<Context>) -> Action {
    Action::requeue(ctx.status.interval)
}

pub async fn loop_forever(args: super::Args, kube: Client) -> Result<()> {
    let api: Api<PoolClaimCrd> = match args.operator.namespace.as_deref() {
        Some(ns) => Api::namespaced(kube.clone(), ns),
        None => Api::all(kube.clone()),
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
    let recorder = Recorder::new(kube.clone(), reporter);

    let watcher_config = Config::default();

    let context = Arc::new(Context {
        kube,
        label_lifecycle_pre_start: args.label_pool_claim_lifecycle_pre_start,
        label_parent: args.label_pool_claim_parent,
        label_pool_parent: args.label_pool_parent,
        label_priority: args.label_pool_claim_priority,
        label_weight: args.label_pool_claim_weight,
        label_weight_penalty: args.label_pool_claim_weight_penalty,
        label_weight_max: args.label_pool_claim_weight_max,
        label_weight_min: args.label_pool_claim_weight_min,
        post_params,
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
