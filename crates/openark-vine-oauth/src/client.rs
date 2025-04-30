use anyhow::Result;
use async_trait::async_trait;
use http::Method;
use openark_core::client::{Client, RequestCredentials};
use serde::Serialize;

#[cfg_attr(feature = "send", async_trait)]
#[cfg_attr(not(feature = "send"), async_trait(?Send))]
pub trait ClientExt
where
    Self: Clone + Client,
{
    #[inline]
    async fn get_auth_args(&self) -> Result<super::OpenIDClientArgs> {
        let url = {
            #[cfg(debug_assertions)]
            {
                "http://localhost:8888/oauth/oidc".parse::<::url::Url>()?
            }

            #[cfg(not(debug_assertions))]
            {
                self.base_url().join("/oauth/oidc")?
            }
        };

        self.request(RequestCredentials::Include, Method::GET, url)
            .await
    }

    #[inline]
    async fn get_auth_configs(
        &self,
        args: &super::OpenIDClientArgs,
    ) -> Result<super::OpenIDConfiguration> {
        let url = args.oauth_config_url.clone();
        self.request(RequestCredentials::Include, Method::GET, url)
            .await
    }

    #[cfg(feature = "actix-web")]
    #[inline]
    async fn get_auth_token(
        &self,
        args: &super::OpenIDClientArgs,
        configs: &super::OpenIDConfiguration,
        code: &str,
    ) -> Result<super::OpenIDClientToken> {
        #[derive(Debug, Serialize)]
        struct Request<'a> {
            client_id: &'a str,
            client_secret: &'a str,
            grant_type: &'a str,
            redirect_uri: &'a str,
            code: &'a str,
        }

        let url = configs.token_endpoint.clone();
        let json = Request {
            client_id: &args.oauth_client_id,
            client_secret: &args.oauth_client_secret,
            grant_type: "authorization_code",
            redirect_uri: args.oauth_redirect_url.as_str(),
            code,
        };
        self.request_with_form(RequestCredentials::Include, Method::POST, url, &json)
            .await
    }
}

#[cfg_attr(feature = "send", async_trait)]
#[cfg_attr(not(feature = "send"), async_trait(?Send))]
impl<T> ClientExt for T where Self: Clone + Client {}
