use std::collections::BTreeSet;

use k8s_openapi::api::{
    core::v1::{Service, ServiceSpec},
    discovery::v1::{EndpointPort, EndpointSlice},
};
use kube::{
    Api, Client, ResourceExt, Result,
    api::{DeleteParams, ListParams, ObjectMeta, PostParams},
};
use openark_spectrum_api::{histogram::HistogramSettings, spectrum_class::SpectrumClassCrd};
#[cfg(feature = "tracing")]
use tracing::{Level, instrument};

use crate::{
    status::Status,
    targets::{
        WeightedItems,
        service::{
            Context as TargetContext, LABEL_KEY_SELECTOR, get_weighted_endpoints,
            infer_address_type, mirror_spec,
        },
    },
    utils::histogram::Histogram,
};

pub(super) struct Context<'a> {
    pub(super) child_metadata: ObjectMeta,
    pub(super) class: SpectrumClassCrd,
    pub(super) client: &'a ::reqwest::Client,
    pub(super) kube: &'a Client,
    pub(super) label_parent: &'a str,
    pub(super) label_weight: &'a str,
    pub(super) name: &'a str,
    pub(super) namespace: &'a str,
    pub(super) post_params: &'a PostParams,
    pub(super) settings: &'a HistogramSettings,
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
        label_parent,
        label_weight,
        name,
        namespace,
        post_params,
        settings,
        target_metadata,
        target_spec,
    } = ctx;

    // Fetch items
    let address_type = infer_address_type(target_spec.as_ref());
    let WeightedItems { items, weights } = match get_weighted_endpoints(TargetContext {
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

    // Calculate histogram
    let Histogram { data } = match Histogram::build(settings, &items, weights) {
        Ok(hist) => hist,
        Err(error) => return Ok(Err(error)),
    };

    // Poll existing resources
    let list_params = ListParams {
        label_selector: Some(format!("{label_parent}={name}")),
        ..Default::default()
    };

    let api_endpointslices = Api::<EndpointSlice>::namespaced(kube.clone(), namespace);
    let last_endpointslices = api_endpointslices.list(&list_params).await?.items;
    let is_exists_endpointslice = |name| {
        last_endpointslices
            .iter()
            .any(|item| item.metadata.name.as_deref() == Some(name))
    };

    let api_services = Api::<Service>::namespaced(kube.clone(), namespace);
    let last_services = api_services.list(&list_params).await?.items;
    let is_exists_service = |name| {
        last_services
            .iter()
            .any(|item| item.metadata.name.as_deref() == Some(name))
    };

    // Poll children names
    let children_names: BTreeSet<_> = (0..data.len())
        .map(|index| format!("{name}-{index}"))
        .collect();

    // Store histogram
    for (index, (child_name, endpoints)) in children_names.iter().zip(data).enumerate() {
        let metadata = ObjectMeta {
            name: Some(child_name.clone()),
            labels: Some({
                let mut map = child_metadata.labels.clone().unwrap_or_default();
                map.insert(LABEL_KEY_SELECTOR.into(), child_name.clone());
                map.insert(label_weight.into(), index.to_string());
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

        // Create services
        let item = Service {
            metadata: metadata.clone(),
            spec: mirror_spec(target_spec.clone()),
            status: None,
        };

        if is_exists_service(child_name) {
            api_services.replace(child_name, post_params, &item).await?
        } else {
            api_services.create(post_params, &item).await?
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
    for item in last_services {
        let name = item.name_any();
        if !children_names.contains(&name) {
            api_services.delete(&name, &delete_params).await?;
        }
    }
    Ok(Ok(()))
}
