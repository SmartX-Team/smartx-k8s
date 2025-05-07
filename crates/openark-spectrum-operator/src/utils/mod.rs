pub(crate) mod histogram;
pub(crate) mod pool;

use std::fmt;

use anyhow::{Result, anyhow};
use k8s_openapi::api::core::v1::Service;
use kube::{Api, Error, ResourceExt, api::PostParams};
use openark_spectrum_api::{
    common::{ObjectReference, ServiceReference},
    pool::PoolCrd,
    spectrum_class::{SpectrumClassCrd, SpectrumClassSpec},
};

use serde::{Serialize, de::DeserializeOwned};
#[cfg(feature = "tracing")]
use tracing::{Level, instrument};
use url::Url;

use crate::status::{Reason, Status};

fn build_service_reference_url_by_raw(name: &str, namespace: &str, port: u16) -> Result<Url> {
    format!("http://{name}.{namespace}.svc:{port}")
        .parse()
        .map_err(|error| anyhow!("Failed to parse service URL: {error}"))
}

pub(crate) fn build_service_reference_url_by_class(class: &SpectrumClassCrd) -> Result<Url> {
    let SpectrumClassCrd {
        spec:
            SpectrumClassSpec {
                backend_ref:
                    ServiceReference {
                        object:
                            ObjectReference {
                                kind,
                                name,
                                namespace,
                                ..
                            },
                        port,
                    },
                ..
            },
        ..
    } = class;

    let namespace = namespace
        .as_deref()
        .ok_or_else(|| anyhow!("Required backend namespace: {kind}/???/{name}"))?;
    let port = *port;
    build_service_reference_url_by_raw(name, namespace, port)
}

pub(crate) fn build_service_reference_url_by_service(svc: &Service, port: u16) -> Result<Url> {
    let name = svc.name_any();
    let namespace = svc.namespace().expect("namespaced resource");
    build_service_reference_url_by_raw(&name, &namespace, port)
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
pub(crate) async fn get_class(
    api: &Api<SpectrumClassCrd>,
    name: &str,
) -> Result<Result<SpectrumClassCrd, Status>, Error> {
    if name.trim().is_empty() {
        return Ok(Err(Status {
            reason: Reason::InvalidClass,
            message: "Empty class".into(),
            requeue: false,
        }));
    }

    match api.get_opt(name).await? {
        Some(class) => {
            if class
                .status
                .as_ref()
                .is_some_and(|status| status.is_accepted())
            {
                Ok(Ok(class))
            } else {
                return Ok(Err(Status {
                    reason: Reason::Pending,
                    message: format!("Class is not accepted: {name}"),
                    requeue: true,
                }));
            }
        }
        None => {
            return Ok(Err(Status {
                reason: Reason::InvalidClass,
                message: format!("Class not found: {name}"),
                requeue: true,
            }));
        }
    }
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
pub(crate) async fn get_pool(
    api: &Api<PoolCrd>,
    name: &str,
) -> Result<Result<PoolCrd, Status>, Error> {
    if name.trim().is_empty() {
        return Ok(Err(Status {
            reason: Reason::InvalidPool,
            message: "Empty pool".into(),
            requeue: false,
        }));
    }

    match api.get_opt(name).await? {
        Some(pool) => {
            if pool
                .status
                .as_ref()
                .is_some_and(|status| status.is_accepted())
            {
                Ok(Ok(pool))
            } else {
                return Ok(Err(Status {
                    reason: Reason::Pending,
                    message: format!("Pool is not accepted: {name}"),
                    requeue: true,
                }));
            }
        }
        None => {
            return Ok(Err(Status {
                reason: Reason::InvalidPool,
                message: format!("Pool not found: {name}"),
                requeue: true,
            }));
        }
    }
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
pub(crate) async fn update_or_create_resource<K>(
    api: &Api<K>,
    post_params: &PostParams,
    name: &str,
    item: &K,
) -> Result<K, Error>
where
    K: Clone + fmt::Debug + Serialize + DeserializeOwned,
{
    match api.get_metadata_opt(name).await? {
        Some(_) => api.replace(name, post_params, item).await,
        None => api.create(post_params, item).await,
    }
}
