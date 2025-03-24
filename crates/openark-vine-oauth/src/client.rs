use anyhow::Result;
use async_trait::async_trait;
use http::Method;
use serde::{Serialize, de::DeserializeOwned};
use url::Url;

pub enum Payload<'a, T> {
    Empty,
    Form(&'a T),
    Json(&'a T),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RequestCredentials {
    Omit,
    Include,
}

#[cfg(target_arch = "wasm32")]
impl From<RequestCredentials> for ::web_sys::RequestCredentials {
    #[inline]
    fn from(value: RequestCredentials) -> Self {
        match value {
            RequestCredentials::Omit => ::web_sys::RequestCredentials::Omit,
            RequestCredentials::Include => ::web_sys::RequestCredentials::Include,
        }
    }
}

#[cfg_attr(feature = "send", async_trait)]
#[cfg_attr(not(feature = "send"), async_trait(?Send))]
pub trait Client {
    fn base_url(&self) -> Url;

    #[inline]
    async fn request<T>(
        &self,
        credentials: RequestCredentials,
        method: Method,
        url: Url,
    ) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.request_with_payload(credentials, method, url, Payload::<()>::Empty)
            .await
    }

    #[inline]
    async fn request_with_form<I, O>(
        &self,
        credentials: RequestCredentials,
        method: Method,
        url: Url,
        form: &I,
    ) -> Result<O>
    where
        I: Sync + Serialize,
        O: DeserializeOwned,
    {
        self.request_with_payload(credentials, method, url, Payload::Form(form))
            .await
    }

    #[inline]
    async fn request_with_json<I, O>(
        &self,
        credentials: RequestCredentials,
        method: Method,
        url: Url,
        json: &I,
    ) -> Result<O>
    where
        I: Sync + Serialize,
        O: DeserializeOwned,
    {
        self.request_with_payload(credentials, method, url, Payload::Json(json))
            .await
    }

    async fn request_with_payload<I, O>(
        &self,
        credentials: RequestCredentials,
        method: Method,
        url: Url,
        payload: Payload<'_, I>,
    ) -> Result<O>
    where
        I: Sync + Serialize,
        O: DeserializeOwned;
}

#[cfg(feature = "reqwest")]
#[cfg_attr(feature = "send", async_trait)]
#[cfg_attr(not(feature = "send"), async_trait(?Send))]
impl Client for ::reqwest::Client {
    fn base_url(&self) -> Url {
        unimplemented!("Implementing Client for ::reqwest::Client is not completed yet")
    }

    async fn request_with_payload<I, O>(
        &self,
        _credentials: RequestCredentials,
        method: Method,
        url: Url,
        payload: Payload<'_, I>,
    ) -> Result<O>
    where
        I: Sync + Serialize,
        O: DeserializeOwned,
    {
        use anyhow::Error;

        let request = self.request(method, url.as_str());
        let request = match payload {
            Payload::Empty => request,
            Payload::Form(form) => request.form(form),
            Payload::Json(json) => request.json(json),
        };

        let response = request.send().await?;
        let status = response.status();

        if status.is_success() {
            match response.json().await {
                Ok(json) => Ok(json),
                Err(error) => Err(Error::from(error)),
            }
        } else {
            match response.text().await.ok() {
                Some(message) if !message.is_empty() => Err(Error::msg(message)),
                Some(_) | None => Err(Error::msg(status.to_string())),
            }
        }
    }
}

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
                "http://localhost:8888/oauth/oidc".parse::<Url>()?
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
