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
    metrics_class::MetricsClassCrd,
    pool::{PoolCrd, PoolSpec},
};
#[cfg(feature = "tracing")]
use tracing::{Level, info, instrument};
use url::Url;

use crate::{
    status::{Reason, Status},
    targets::{Target, get_target},
    utils::get_metrics_class,
};

struct Context {
    api_class: Api<MetricsClassCrd>,
    client: ::reqwest::Client,
    kube: Client,
    label_claim_parent: String,
    label_parent: String,
    pool_base_url: Url,
    post_params: PostParams,
    status: ::openark_core::operator::Context,
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
async fn reconcile(pool: Arc<PoolCrd>, ctx: Arc<Context>) -> Result<Action, Error> {
    let metadata = &pool.metadata;
    let name = pool.name_any();
    let namespace = metadata.namespace.as_deref().expect("Namespaced resource");
    let PoolSpec {
        metrics_class_name,
        target_ref,
    } = &pool.spec;

    // Define status update function
    let api = Api::namespaced(ctx.kube.clone(), namespace);
    let commit = |status: Status| {
        let api = &api;
        let object = &*pool;
        ctx.status.commit(api, object, status)
    };

    // Define accept function
    let commit_ok = || {
        commit(Status {
            reason: Reason::Accepted,
            message: "Valid Pool".into(),
            requeue: true,
        })
    };

    // Validate class
    let class = match get_metrics_class(&ctx.api_class, metrics_class_name).await? {
        Ok(class) => class,
        Err(action) => return commit(action).await,
    };

    // Validate target
    let (target_metadata, target_spec) = match get_target(&ctx.kube, namespace, target_ref).await? {
        Ok(list) => list,
        Err(action) => return commit(action).await,
    };

    // Build owner references
    let child_owner_references = vec![OwnerReference {
        block_owner_deletion: Some(true),
        controller: Some(false),
        ..pool
            .owner_ref(&Default::default())
            .expect("pool owner reference")
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

    // Create/Update target resources
    match match target_spec {
        Target::Service(target_spec) => {
            let child_ctx = self::service::Context {
                child_metadata,
                class,
                client: &ctx.client,
                kube: &ctx.kube,
                label_claim_parent: &ctx.label_claim_parent,
                label_parent: &ctx.label_parent,
                name: &name,
                namespace,
                pool_url: &ctx.pool_base_url,
                post_params: &ctx.post_params,
                target_metadata,
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

fn error_policy(_pool: Arc<PoolCrd>, _error: &Error, ctx: Arc<Context>) -> Action {
    Action::requeue(ctx.status.interval)
}

pub async fn loop_forever(
    args: super::Args,
    client: ::reqwest::Client,
    kube: Client,
) -> Result<()> {
    let api: Api<PoolCrd> = match args.operator.namespace.as_deref() {
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
        kube,
        label_claim_parent: args.label_pool_claim_parent,
        label_parent: args.label_pool_parent,
        pool_base_url: args.pool_base_url,
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
