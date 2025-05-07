use k8s_openapi::api::core::v1::{Service, ServiceSpec};
use kube::{
    Api, Client, Result,
    api::{ListParams, ObjectMeta, PostParams},
};
use openark_spectrum_api::spectrum_class::SpectrumClassCrd;
#[cfg(feature = "tracing")]
use tracing::{Level, instrument};

use crate::{
    status::{Reason, Status},
    targets::service::{Context as TargetContext, get_weighted_endpoints, infer_address_type},
    utils::pool::{Item, Lifecycle, Resource, schedule},
};

pub(super) struct Context<'a> {
    pub(super) child_metadata: ObjectMeta,
    pub(super) class: SpectrumClassCrd,
    pub(super) client: &'a ::reqwest::Client,
    pub(super) kube: &'a Client,
    pub(super) label_claim_parent: &'a str,
    pub(super) label_lifecycle_post_stop: &'a str,
    pub(super) label_lifecycle_pre_start: &'a str,
    pub(super) label_parent: &'a str,
    pub(super) label_priority: &'a str,
    pub(super) label_weight: &'a str,
    pub(super) label_weight_max: &'a str,
    pub(super) label_weight_min: &'a str,
    pub(super) name: &'a str,
    pub(super) namespace: &'a str,
    pub(super) post_params: &'a PostParams,
    pub(super) target_metadata: ObjectMeta,
    pub(super) target_spec: Option<ServiceSpec>,
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
pub(super) async fn update_resources(ctx: Context<'_>) -> Result<Result<(), Status>> {
    let Context {
        child_metadata,
        class,
        client,
        kube,
        label_claim_parent,
        label_lifecycle_post_stop,
        label_lifecycle_pre_start,
        label_parent,
        label_priority,
        label_weight,
        label_weight_max,
        label_weight_min,
        name,
        namespace,
        post_params,
        target_metadata,
        target_spec,
    } = ctx;

    // Fetch items
    let address_type = infer_address_type(target_spec.as_ref());
    let resources = match get_weighted_endpoints(TargetContext {
        address_type,
        child_metadata: &child_metadata,
        class: &class,
        client,
        kube,
        target_metadata: &target_metadata,
    })
    .await?
    {
        Ok(items) => items,
        Err(error) => return Ok(Err(error)),
    };

    // Fetch all claimed resources
    let api_services = Api::<Service>::namespaced(kube.clone(), namespace);
    let list_params = ListParams {
        label_selector: Some(format!("{label_parent}={name}")),
        ..Default::default()
    };
    let services = api_services.list(&list_params).await?.items;

    // Extract claim name, lifecycle, priority, weight metadata
    let items = services
        .iter()
        .filter_map(|item| {
            let annotations = item.metadata.annotations.as_ref()?;
            let labels = item.metadata.labels.as_ref()?;

            Some(Item {
                claim_name: labels.get(label_claim_parent)?,
                lifecycle: Lifecycle {
                    pre_start: labels.get(label_lifecycle_pre_start)?.parse().ok()?,
                    post_stop: labels.get(label_lifecycle_post_stop)?.parse().ok()?,
                },
                resource: Resource {
                    priority: labels.get(label_priority)?.parse().ok()?,
                    min: annotations
                        .get(label_weight_min)
                        .and_then(|e| e.parse().ok()),
                    max: annotations
                        .get(label_weight_max)
                        .and_then(|e| e.parse().ok()),
                    weight: labels.get(label_weight)?.parse().ok()?,
                },
                item,
            })
        })
        .collect();

    // Allocate resources into items
    let items = match schedule(items, resources) {
        Ok(items) => items,
        Err(error) => {
            return Ok(Err(Status {
                reason: Reason::InvalidPool,
                message: format!("Failed to schedule: {error}"),
                requeue: true,
            }));
        }
    };

    // FIXME: Store pool
    // FIXME: Create endpointslices
    // FIXME: Trigger lifecycle
    // FIXME:   - Create a connection pool for long time jobs
    // FIXME: Delete unmanaged resources
    todo!()
}
