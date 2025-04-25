use anyhow::Result;
use async_trait::async_trait;
use http::Method;
use openark_vine_oauth::client::{Client, RequestCredentials};
use url::Url;

use crate::{command::SessionCommandView, exec::ExecArgs};

#[cfg_attr(feature = "send", async_trait)]
#[cfg_attr(not(feature = "send"), async_trait(?Send))]
pub trait ClientExt
where
    Self: Clone + Client,
{
    #[inline]
    async fn vine_session_command_list(&self, base_url: Url) -> Result<Vec<SessionCommandView>> {
        let url = base_url.join("commands")?;
        self.request(RequestCredentials::Include, Method::GET, url)
            .await
    }

    #[inline]
    async fn vine_session_exec(&self, base_url: Url, args: &ExecArgs) -> Result<()> {
        let url = base_url.join("exec")?;
        self.request_with_json(RequestCredentials::Include, Method::POST, url, args)
            .await
    }
}

#[cfg_attr(feature = "send", async_trait)]
#[cfg_attr(not(feature = "send"), async_trait(?Send))]
impl<T> ClientExt for T where Self: Clone + Client {}
