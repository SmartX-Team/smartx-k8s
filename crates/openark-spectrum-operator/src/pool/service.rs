use std::collections::BTreeSet;

use k8s_openapi::api::{
    core::v1::{Service, ServiceSpec},
    discovery::v1::{EndpointPort, EndpointSlice},
};
use kube::{
    Api, Client, ResourceExt, Result,
    api::{DeleteParams, ListParams, ObjectMeta, PostParams},
};
use openark_spectrum_api::{
    client::PoolClient, schema::ScheduledItem, spectrum_class::SpectrumClassCrd,
};
#[cfg(feature = "tracing")]
use tracing::{Level, instrument};
use url::Url;

use crate::{
    status::{Reason, Status},
    targets::service::{
        Context as TargetContext, LABEL_KEY_SELECTOR, get_weighted_endpoints, infer_address_type,
    },
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
    pub(super) label_weight_penalty: &'a str,
    pub(super) label_weight_max: &'a str,
    pub(super) label_weight_min: &'a str,
    pub(super) name: &'a str,
    pub(super) namespace: &'a str,
    pub(super) pool_url: &'a Url,
    pub(super) post_params: &'a PostParams,
    pub(super) target_metadata: ObjectMeta,
    pub(super) target_spec: Option<ServiceSpec>,
}

#[cfg_attr(feature = "tracing", instrument(
    level = Level::INFO,
    skip_all,
    fields(name = %ctx.name, namespace = %ctx.namespace),
))]
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
        label_weight_penalty,
        label_weight_max,
        label_weight_min,
        name,
        namespace,
        pool_url,
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
    let items: Vec<_> = services
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
                    penalty: annotations
                        .get(label_weight_penalty)
                        .and_then(|e| e.parse().ok()),
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

    // Fetch binding states
    let binded = match client
        .get_service_binding_states(pool_url.clone(), &resources)
        .await
    {
        Ok(items) => items,
        Err(error) => {
            return Ok(Err(Status {
                reason: Reason::InvalidPool,
                message: format!("Failed to fetch binding states: {error}"),
                requeue: true,
            }));
        }
    };

    // Map binding states
    let binded = binded
        .into_iter()
        .map(|claim_name| {
            claim_name.as_deref().and_then(|claim_name| {
                items
                    .iter()
                    .enumerate()
                    .find(|(_, item)| claim_name == item.claim_name)
                    .map(|(index, _)| index)
            })
        })
        .collect();

    // Allocate resources into items
    let items = match schedule(items, binded, resources) {
        Ok(items) => items,
        Err(error) => {
            return Ok(Err(Status {
                reason: Reason::InvalidPool,
                message: format!("Failed to allocate resources: {error}"),
                requeue: true,
            }));
        }
    };

    // FIXME: Trigger lifecycle
    // FIXME:   - Create a connection pool for long time jobs
    // FIXME:   - 작업 우선순위: (claim name) 별로 round-robin, 각 claim 에선 priority 가 높은 것부터 처리
    // FIXME:   - 상태 저장: 전용 backend를 만들고 오프로딩
    match client
        .commit_service_binding_states(pool_url.clone(), &items)
        .await
    {
        Ok(()) => (),
        Err(error) => {
            return Ok(Err(Status {
                reason: Reason::InvalidPool,
                message: format!("Failed to commit binding states: {error}"),
                requeue: true,
            }));
        }
    };

    // Poll existing resources
    let api_endpointslices = Api::<EndpointSlice>::namespaced(kube.clone(), namespace);
    let last_endpointslices = api_endpointslices.list(&list_params).await?.items;
    let is_exists_endpointslice = |name| {
        last_endpointslices
            .iter()
            .any(|item| item.metadata.name.as_deref() == Some(name))
    };

    // Poll children names
    let children_names: BTreeSet<_> = items.iter().map(|item| item.item.name_any()).collect();

    // Store pool
    for (
        child_name,
        ScheduledItem {
            item: _,
            resources: endpoints,
        },
    ) in children_names.iter().zip(items)
    {
        let metadata = ObjectMeta {
            name: Some(child_name.clone()),
            labels: Some({
                let mut map = child_metadata.labels.clone().unwrap_or_default();
                map.insert(LABEL_KEY_SELECTOR.into(), child_name.clone());
                map.insert(label_claim_parent.into(), child_name.clone());
                map
            }),
            ..child_metadata.clone()
        };

        // Create endpointslices
        let item = EndpointSlice {
            address_type: address_type.into(),
            // TODO: Add support for paged endpointslices (2 or more endpointslices) to reduce kubernetes API load?
            // TODO: One way to recycle endpointslices to reduce Kubernetes API load is to periodically collect
            // TODO:  **stable** endpoints and commit them to a single endpointslice.
            endpoints,
            metadata: metadata.clone(),
            // Convert ServicePort into EndpointPort
            ports: target_spec
                .as_ref()
                .and_then(|spec| spec.ports.as_ref())
                .map(|ports| {
                    ports
                        .iter()
                        .map(|port| EndpointPort {
                            app_protocol: port.app_protocol.clone(),
                            name: port.name.clone(),
                            port: Some(port.port),
                            protocol: port.protocol.clone(),
                        })
                        .collect()
                }),
        };

        if is_exists_endpointslice(child_name) {
            api_endpointslices
                .replace(child_name, post_params, &item)
                .await?
        } else {
            api_endpointslices.create(post_params, &item).await?
        };
    }

    // Delete unmanaged resources
    let delete_params = DeleteParams::default();
    for item in last_endpointslices {
        let name = item.name_any();
        if !children_names.contains(&name) {
            api_endpointslices.delete(&name, &delete_params).await?;
        }
    }
    Ok(Ok(()))
}
