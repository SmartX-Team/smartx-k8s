use std::{borrow::Cow, collections::BTreeSet};

use k8s_openapi::api::{
    core::v1::{Service, ServiceSpec},
    discovery::v1::{Endpoint, EndpointPort, EndpointSlice},
};
use kube::{
    Api, Client, Error, ResourceExt, Result,
    api::{DeleteParams, ListParams, ObjectMeta, PostParams},
    core::ErrorResponse,
};
use openark_histogram_api::{
    client::ClientExt,
    histogram::HistogramSettings,
    histogram_class::HistogramClassCrd,
    schema::{WeightRequest, WeightResponse},
};
#[cfg(feature = "tracing")]
use tracing::{Level, instrument};

use crate::utils::{Histogram, build_service_reference_url_by_class};

const LABEL_KEY_SELECTOR: &str = "kubernetes.io/service-name";
const LABEL_KEY_WEIGHT: &str = "org.ulagbulag.io/histogram-weight";

#[must_use]
pub(super) fn mirror_spec(spec: Option<ServiceSpec>) -> Option<ServiceSpec> {
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
async fn load_endpoints(
    api: &Api<EndpointSlice>,
    address_type: &str,
    target_name: &str,
) -> Result<Vec<Endpoint>> {
    // Load slices
    let lp = ListParams {
        label_selector: Some(format!("{LABEL_KEY_SELECTOR}={target_name}")),
        ..Default::default()
    };
    let slices = api.list(&lp).await?.items;

    // Collect endpoints
    Ok(slices
        .into_iter()
        .filter(|slice| &slice.address_type == address_type)
        .flat_map(|slice| slice.endpoints)
        .collect())
}

pub(super) struct Context<'a> {
    pub(super) address_type: &'a str,
    pub(super) child_metadata: &'a ObjectMeta,
    pub(super) class: &'a HistogramClassCrd,
    pub(super) client: &'a ::reqwest::Client,
    pub(super) kube: &'a Client,
    pub(super) label_parent: &'a str,
    pub(super) name: &'a str,
    pub(super) namespace: &'a str,
    pub(super) post_params: &'a PostParams,
    pub(super) settings: &'a HistogramSettings,
    pub(super) target_metadata: &'a ObjectMeta,
    pub(super) target_spec: Option<&'a ServiceSpec>,
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
pub(super) async fn update_subresources(
    ctx: Context<'_>,
) -> Result<Result<(), super::ValidationError>> {
    let Context {
        address_type,
        child_metadata,
        class,
        client,
        kube,
        label_parent,
        name,
        namespace,
        post_params,
        settings,
        target_metadata,
        target_spec,
    } = ctx;

    let target_name = target_metadata.name.as_deref().unwrap();
    let target_namespace = target_metadata
        .namespace
        .as_deref()
        .expect("Namespaced resource");

    let target_api = Api::<EndpointSlice>::namespaced(kube.clone(), target_namespace);
    let list = load_endpoints(&target_api, address_type, &target_name).await?;

    // Do nothing if the endpoints are not provisioned yet
    if list.is_empty() {
        return Ok(Ok(()));
    }

    let url = match build_service_reference_url_by_class(class) {
        Ok(url) => url,
        Err(error) => {
            return Ok(Err(super::ValidationError {
                reason: "InvalidClass".into(),
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
        list: Cow::Borrowed(list.as_slice()),
    };

    let WeightResponse { weights } =
        client
            .get_service_weights(url, &args)
            .await
            .map_err(|error| {
                Error::Api(ErrorResponse {
                    code: 500,
                    status: "InternalError".into(),
                    reason: "InternalError".into(),
                    message: error.to_string(),
                })
            })?;

    // Validate weights
    if list.len() != weights.len() {
        return Ok(Err(super::ValidationError {
            reason: "InvalidClass".into(),
            message: "Invalid weights".into(),
            requeue: true,
        }));
    }

    // Calculate histogram
    let Histogram { data } = match Histogram::build(settings, &list, weights) {
        Ok(hist) => hist,
        Err(error) => {
            return Ok(Err(super::ValidationError {
                reason: "InvalidHistogram".into(),
                message: error.to_string(),
                requeue: true,
            }));
        }
    };

    // Poll existing resources
    let list_params = ListParams {
        label_selector: Some(format!("{label_parent}={name}")),
        ..Default::default()
    };

    let api_endpointslices = Api::<EndpointSlice>::namespaced(kube.clone(), &namespace);
    let last_endpointslices = api_endpointslices.list(&list_params).await?.items;
    let is_exists_endpointslice = |name| {
        last_endpointslices
            .iter()
            .any(|item| item.metadata.name.as_deref() == Some(name))
    };

    let api_services = Api::<Service>::namespaced(kube.clone(), &namespace);
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
                map.insert(LABEL_KEY_WEIGHT.into(), index.to_string());
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
                .replace(child_name, &post_params, &item)
                .await?
        } else {
            api_endpointslices.create(&post_params, &item).await?
        };

        // Create services
        let item = Service {
            metadata: metadata.clone(),
            spec: mirror_spec(target_spec.cloned()),
            status: None,
        };
        if is_exists_service(child_name) {
            api_services
                .replace(child_name, &post_params, &item)
                .await?
        } else {
            api_services.create(&post_params, &item).await?
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
