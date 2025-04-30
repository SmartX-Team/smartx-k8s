use anyhow::Result;
use async_trait::async_trait;
use http::Method;
use k8s_openapi::api::discovery::v1::Endpoint;
use openark_core::client::{Client, RequestCredentials};
use url::Url;

use crate::schema::{WeightRequest, WeightResponse};

#[cfg_attr(feature = "send", async_trait)]
#[cfg_attr(not(feature = "send"), async_trait(?Send))]
pub trait ClientExt
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
impl<T> ClientExt for T where Self: Clone + Client {}
