use std::borrow::Cow;

use k8s_openapi::api::{
    core::v1::ServiceSpec,
    discovery::v1::{Endpoint, EndpointSlice},
};
use kube::{
    Api, Client, Result,
    api::{ListParams, ObjectMeta},
};
use openark_spectrum_api::{
    client::BackendClient,
    schema::{WeightRequest, WeightResponse},
    spectrum_class::SpectrumClassCrd,
};
#[cfg(feature = "tracing")]
use tracing::{Level, instrument};

use crate::{
    status::{Reason, Status},
    utils::build_service_reference_url_by_class,
};

pub(crate) const LABEL_KEY_SELECTOR: &str = "kubernetes.io/service-name";

/// It will first probe IPv4, and only select IPv6
/// if IPv6 is explicitly declared instead of IPv4.
#[must_use]
pub(crate) fn infer_address_type(spec: Option<&ServiceSpec>) -> &str {
    if spec.is_none_or(|spec| {
        spec.ip_families
            .as_ref()
            .is_none_or(|ip_families| ip_families.iter().any(|family| family == "IPv4"))
    }) {
        "IPv4"
    } else {
        "IPv6"
    }
}

#[must_use]
pub(crate) fn mirror_spec(spec: Option<ServiceSpec>) -> Option<ServiceSpec> {
    spec.map(|spec| ServiceSpec {
        cluster_ip: None,
        cluster_ips: None,
        external_ips: None,
        external_name: None,
        load_balancer_ip: None,
        publish_not_ready_addresses: None,
        selector: None,
        ..spec
    })
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
pub(crate) async fn get_endpoints(
    api: &Api<EndpointSlice>,
    address_type: &str,
    list_params: &ListParams,
) -> Result<Vec<Endpoint>> {
    // Load slices
    let slices = api.list(list_params).await?.items;

    // Collect endpoints
    Ok(slices
        .into_iter()
        .filter(|slice| slice.address_type == address_type)
        .flat_map(|slice| slice.endpoints)
        .collect())
}

pub(crate) struct Context<'a> {
    pub(crate) address_type: &'a str,
    pub(crate) child_metadata: &'a ObjectMeta,
    pub(crate) class: &'a SpectrumClassCrd,
    pub(crate) client: &'a ::reqwest::Client,
    pub(crate) kube: &'a Client,
    pub(crate) target_metadata: &'a ObjectMeta,
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
pub(crate) async fn get_weighted_endpoints(
    ctx: Context<'_>,
) -> Result<Result<super::WeightedItems<Endpoint>, Status>> {
    let Context {
        address_type,
        child_metadata,
        class,
        client,
        kube,
        target_metadata,
    } = ctx;

    let target_name = target_metadata.name.as_deref().unwrap();
    let target_namespace = target_metadata
        .namespace
        .as_deref()
        .expect("Namespaced resource");

    // Validate items
    let target_api = Api::<EndpointSlice>::namespaced(kube.clone(), target_namespace);
    let target_list_params = ListParams {
        label_selector: Some(format!("{LABEL_KEY_SELECTOR}={target_name}")),
        ..Default::default()
    };
    let items = get_endpoints(&target_api, address_type, &target_list_params).await?;

    // Fetch weights
    let url = match build_service_reference_url_by_class(class) {
        Ok(url) => url,
        Err(error) => {
            return Ok(Err(Status {
                reason: Reason::InvalidClass,
                message: error.to_string(),
                requeue: true,
            }));
        }
    };
    let args = WeightRequest {
        metadata: ObjectMeta {
            name: Some(target_name.into()),
            namespace: Some(target_namespace.into()),
            annotations: child_metadata.annotations.clone(),
            labels: child_metadata.labels.clone(),
            ..Default::default()
        },
        list: Cow::Borrowed(items.as_slice()),
    };

    let WeightResponse { weights } = match client.get_service_weights(url, &args).await {
        Ok(response) => response,
        Err(error) => {
            return Ok(Err(Status {
                reason: Reason::ProvisioningError,
                message: format!("Failed to get service weights: {error}"),
                requeue: true,
            }));
        }
    };

    // Validate weights
    if items.len() != weights.len() {
        return Ok(Err(Status {
            reason: Reason::InvalidClass,
            message: "Invalid weights".into(),
            requeue: true,
        }));
    }

    Ok(Ok(super::WeightedItems { items, weights }))
}
