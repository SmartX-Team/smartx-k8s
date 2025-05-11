pub(crate) mod service;

use k8s_openapi::{
    Resource,
    api::core::v1::{Service, ServiceSpec},
};
use kube::{Api, Client, Result, api::ObjectMeta};
use openark_spectrum_api::common::ObjectReference;
use ordered_float::OrderedFloat;
#[cfg(feature = "tracing")]
use tracing::{Level, instrument};

use crate::status::{Reason, Status};

pub(crate) enum Target {
    Service(Option<ServiceSpec>),
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
pub(crate) async fn get_target(
    client: &Client,
    namespace: &str,
    target_ref: &ObjectReference,
) -> Result<Result<(ObjectMeta, Target), Status>> {
    let ObjectReference {
        group,
        kind,
        name,
        namespace: target_namespace,
    } = target_ref;

    let namespace = target_namespace.as_deref().unwrap_or(namespace);
    match (group.as_str(), kind.as_str()) {
        (Service::GROUP, Service::KIND) => {
            let api = Api::<Service>::namespaced(client.clone(), namespace);
            match api.get_opt(name).await? {
                Some(item) => Ok(Ok((item.metadata, Target::Service(item.spec)))),
                None => Ok(Err(Status {
                    reason: Reason::InvalidTarget,
                    message: format!("Target not found: {kind}/{namespace}/{name}"),
                    requeue: true,
                })),
            }
        }
        (group, kind) => Ok(Err(Status {
            reason: Reason::InvalidTarget,
            message: if group.is_empty() {
                format!("Unsupported target kind: {kind}")
            } else {
                format!("Unsupported target kind: {group}/{kind}")
            },
            requeue: false,
        })),
    }
}

#[derive(Clone, Debug)]
pub(crate) struct WeightedItems<T> {
    pub(crate) items: Vec<T>,
    pub(crate) weights: Vec<Option<OrderedFloat<f64>>>,
}
