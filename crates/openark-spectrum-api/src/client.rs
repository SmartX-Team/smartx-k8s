use anyhow::Result;
use async_trait::async_trait;
use http::Method;
use k8s_openapi::api::discovery::v1::Endpoint;
use openark_core::client::{Client, RequestCredentials};
use url::Url;

use crate::schema::{PoolCommitRequest, PoolRequest, PoolResponse, WeightRequest, WeightResponse};

#[cfg_attr(feature = "send", async_trait)]
#[cfg_attr(not(feature = "send"), async_trait(?Send))]
pub trait BackendClient
where
    Self: Clone + Client,
{
    #[inline]
    async fn get_service_weights(
        &self,
        url: Url,
        args: &WeightRequest<Endpoint>,
    ) -> Result<WeightResponse> {
        let url = url.join("v1/Service")?;
        self.request_with_json(RequestCredentials::Include, Method::POST, url, args)
            .await
    }
}

#[cfg_attr(feature = "send", async_trait)]
#[cfg_attr(not(feature = "send"), async_trait(?Send))]
impl<T> BackendClient for T where Self: Clone + Client {}

#[cfg_attr(feature = "send", async_trait)]
#[cfg_attr(not(feature = "send"), async_trait(?Send))]
pub trait PoolClient
where
    Self: Clone + Client,
{
    #[inline]
    async fn commit_service_binding_states(
        &self,
        url: Url,
        args: &PoolCommitRequest,
    ) -> Result<()> {
        let url = url.join("v1/Service/commit")?;
        self.request_with_json(RequestCredentials::Include, Method::POST, url, args)
            .await
    }

    #[inline]
    async fn get_service_binding_states(
        &self,
        url: Url,
        args: &PoolRequest,
    ) -> Result<PoolResponse> {
        let url = url.join("v1/Service")?;
        self.request_with_json(RequestCredentials::Include, Method::POST, url, args)
            .await
    }
}

#[cfg_attr(feature = "send", async_trait)]
#[cfg_attr(not(feature = "send"), async_trait(?Send))]
impl<T> PoolClient for T where Self: Clone + Client {}
