use anyhow::Result;
use async_trait::async_trait;
use http::Method;
use openark_core::client::{Client, RequestCredentials};
use url::Url;

use crate::{file::FileEntry, global::GlobalConfiguration};

#[cfg_attr(feature = "send", async_trait)]
#[cfg_attr(not(feature = "send"), async_trait(?Send))]
pub trait ClientExt
where
    Self: Clone + Client,
{
    /// Returns a file content [`Url`].
    ///
    #[inline]
    fn get_file_content_url(&self, path: &str) -> Result<Url, ::url::ParseError> {
        self.base_url()
            .join(&format!("data/{}", path.trim_start_matches('/')))
    }

    /// Returns a [`FileEntry`].
    ///
    #[inline]
    async fn get_file_entry(&self, path: &str) -> Result<Option<FileEntry>> {
        let url = self
            .base_url()
            .join(&format!("metadata/{}", path.trim_start_matches('/')))?;
        self.request(RequestCredentials::Include, Method::GET, url)
            .await
    }

    /// Returns a browser's [`GlobalConfiguration`].
    ///
    #[inline]
    async fn get_global_conf(&self) -> Result<Option<GlobalConfiguration>> {
        let url = self.base_url();
        self.request(RequestCredentials::Include, Method::GET, url)
            .await
    }
}

#[cfg_attr(feature = "send", async_trait)]
#[cfg_attr(not(feature = "send"), async_trait(?Send))]
impl<T> ClientExt for T where Self: Clone + Client {}
