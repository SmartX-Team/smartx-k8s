use k8s_openapi::api::core::v1::{Service, ServiceSpec};
use kube::{
    Api, Client, Result,
    api::{ObjectMeta, PostParams},
};
#[cfg(feature = "tracing")]
use tracing::{Level, instrument};

use crate::{targets::service::mirror_spec, utils::update_or_create_resource};

pub(super) struct Context<'a> {
    pub(super) child_metadata: ObjectMeta,
    pub(super) kube: &'a Client,
    pub(super) name: &'a str,
    pub(super) namespace: &'a str,
    pub(super) post_params: &'a PostParams,
    pub(super) target_spec: Option<ServiceSpec>,
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
pub(super) async fn update_resources(ctx: Context<'_>) -> Result<Result<(), super::Status>> {
    let Context {
        child_metadata,
        kube,
        name,
        namespace,
        post_params,
        target_spec,
    } = ctx;

    // Create service
    {
        let api = Api::namespaced(kube.clone(), namespace);
        let spec = mirror_spec(target_spec);
        let item = Service {
            metadata: child_metadata.clone(),
            spec,
            status: None,
        };
        update_or_create_resource(&api, post_params, name, &item).await?;
    }
    Ok(Ok(()))
}
