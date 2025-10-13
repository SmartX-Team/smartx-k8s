use anyhow::Result;
use async_trait::async_trait;
use http::Method;
use openark_core::client::{Client, RequestCredentials};
use openark_vine_oauth::User;
use serde_json::Value;
use url::Url;

use crate::{
    app::App,
    page::{PageRef, PageSpec},
};

#[cfg_attr(feature = "send", async_trait)]
#[cfg_attr(not(feature = "send"), async_trait(?Send))]
pub trait ClientExt
where
    Self: Clone + Client,
{
    #[inline]
    async fn get_app(&self) -> Result<App> {
        let url = self.base_url();
        self.request(RequestCredentials::Include, Method::GET, url)
            .await
    }

    #[inline]
    async fn get_page(&self, page: &PageRef) -> Result<PageSpec> {
        let url = self.base_url().join(&page.to_string())?;
        self.request(RequestCredentials::Include, Method::GET, url)
            .await
    }

    #[inline]
    async fn get_user(&self) -> Result<Option<User>> {
        let url = self.base_url().join("users/me")?;
        self.request(RequestCredentials::Include, Method::GET, url)
            .await
    }

    #[inline]
    async fn get_table_rows(&self, base_url: Url) -> Result<Value> {
        self.request(RequestCredentials::Include, Method::GET, base_url)
            .await
    }

    #[inline]
    async fn update_item(&self, base_url: Url, item: Option<&str>, value: &Value) -> Result<()> {
        let method = match item {
            Some(_) => Method::PATCH,
            None => Method::PUT,
        };
        let url = match item {
            Some(name) => base_url.join(name)?,
            None => base_url,
        };
        self.request_with_json(RequestCredentials::Include, method, url, value)
            .await
    }

    #[inline]
    async fn delete_table_row(&self, base_url: Url, item: String) -> Result<()> {
        let url = base_url.join(&item)?;
        self.request(RequestCredentials::Include, Method::DELETE, url)
            .await
    }
}

#[cfg_attr(feature = "send", async_trait)]
#[cfg_attr(not(feature = "send"), async_trait(?Send))]
impl<T> ClientExt for T where Self: Clone + Client {}
