use std::{fmt, rc::Rc};

use anyhow::{Error, Result};
use async_trait::async_trait;
use gloo_net::http::{Method, RequestBuilder};
use http::header;
use openark_core::client::{Payload, RequestCredentials};
use openark_vine_oauth::{
    State, UserClaims,
    client::ClientExt,
    error::{Error as AuthenticationError, ErrorInvalidRequest, ErrorInvalidToken, ErrorKind},
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use tracing::Level;
use url::Url;
use web_sys::{RequestMode, RequestRedirect};
use yew::{platform::spawn_local, prelude::*};
use yew_router::prelude::*;
use yewdux::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Client;

#[async_trait(?Send)]
impl ::openark_core::client::Client for Client {
    fn base_url(&self) -> Url {
        let base_url = option_env!("API_BASE_URL").unwrap_or("/api/v1/");
        crate::router::href().join(base_url).unwrap()
    }

    async fn request_with_payload<I, O>(
        &self,
        credentials: RequestCredentials,
        method: Method,
        url: Url,
        payload: Payload<'_, I>,
    ) -> Result<O>
    where
        I: ?Sized + Sync + Serialize,
        O: DeserializeOwned,
    {
        // Mockup the api
        #[cfg(debug_assertions)]
        let url = if option_env!("API_BASE_URL").is_none()
            && url.port().unwrap_or(443)
                == ::openark_core::client::Client::base_url(self)
                    .port()
                    .unwrap()
        {
            match method {
                Method::DELETE => format!("{url}/delete.json").parse()?,
                Method::GET => format!("{url}/get.json").parse()?,
                Method::PATCH => format!("{url}/patch.json").parse()?,
                Method::POST => format!("{url}/post.json").parse()?,
                Method::PUT => format!("{url}/put.json").parse()?,
                _ => url,
            }
        } else {
            url
        };

        let request = RequestBuilder::new(url.as_str())
            .method(method.clone())
            .credentials(credentials.into())
            .mode(RequestMode::Cors)
            .redirect(RequestRedirect::Follow);
        let request = match payload {
            Payload::Empty => request.build()?,
            Payload::Form(form) => {
                let body = ::serde_urlencoded::to_string(form)?;
                request
                    .header(
                        header::CONTENT_TYPE.as_str(),
                        "application/x-www-form-urlencoded",
                    )
                    .body(body)?
            }
            Payload::Json(json) => request.json(json)?,
        };
        let response = request.send().await?;

        match response.status() {
            200..302 | 303..400 => {
                // Unpack payload for logging
                #[cfg(debug_assertions)]
                {
                    let json: ::serde_json::Value = match response.json().await {
                        Ok(json) => json,
                        Err(message) => return Err(Error::msg(message)),
                    };
                    ::tracing::debug!("Response: {json:#?}");
                    ::serde_json::from_value(json).map_err(Into::into)
                }

                #[cfg(not(debug_assertions))]
                match response.json().await {
                    Ok(json) => Ok(json),
                    Err(message) => Err(Error::msg(message)),
                }
            }
            // Found
            302 => {
                self.assert_sign_in().await?;
                Box::pin(self.request_with_payload(credentials, method, url, payload)).await
            }
            // Unauthorized
            401 => {
                let error: AuthenticationError = response.text().await?.parse()?;
                match &error.kind {
                    ErrorKind::InvalidRequest(ErrorInvalidRequest::AccessTokenMissing)
                    | ErrorKind::InvalidToken(ErrorInvalidToken::AccessTokenExpired) => {
                        self.assert_sign_in().await?;
                        Box::pin(self.request_with_payload(credentials, method, url, payload)).await
                    }
                    ErrorKind::InvalidToken(ErrorInvalidToken::MalformedJwtToken) => {
                        Err(error.into())
                    }
                }
            }
            // Forbidden | Not Found
            403 | 404 => {
                let json = ::serde_json::from_value(Value::Null)?;
                Ok(json)
            }
            ..200 | 400 | 402 | 404.. => match response.text().await.ok() {
                Some(message) if !message.is_empty() => Err(Error::msg(message)),
                Some(_) | None => Err(Error::msg(response.status_text())),
            },
        }
    }
}

impl Client {
    /// Validate the user information or sign in
    async fn assert_sign_in(&self) -> Result<()> {
        // Try getting user info
        let UserClaims {
            data: user,
            refresh_token,
        } = UserClaims::infer();
        match user {
            Ok(_) => Ok(()),
            Err(
                ErrorKind::InvalidRequest(ErrorInvalidRequest::AccessTokenMissing)
                | ErrorKind::InvalidToken(ErrorInvalidToken::MalformedJwtToken),
            ) => self.sign_in().await,
            Err(ErrorKind::InvalidToken(ErrorInvalidToken::AccessTokenExpired)) => {
                match refresh_token {
                    Some(refresh_token) => self.refresh_access_token(&refresh_token).await,
                    None => self.sign_in().await,
                }
            }
        }
    }

    async fn sign_in(&self) -> Result<()> {
        // Load an OpenID provider configuration
        let args = self.get_auth_args().await?;
        let configs = self.get_auth_configs(&args).await?;

        // Generate a new state
        let state = State::new(crate::router::href());

        // Redirect to the auth page
        let mut url = configs.authorization_endpoint;
        url.set_query(Some(&format!(
            "client_id={client_id}&redirect_uri={redirect_uri}&response_type={response_type}&scope={scope}&state={state}",
            client_id = args.oauth_client_id,
            redirect_uri = args.oauth_redirect_url,
            response_type = "code",
            scope = args.oauth_scopes.replace(",", " "),
        )));
        crate::router::redirect_to(url.as_str())
    }

    async fn refresh_access_token(&self, refresh_token: &str) -> Result<()> {
        // TODO: Refresh the token without re-signing-in
        let _ = refresh_token;
        self.sign_in().await
    }
}

pub(super) struct Request<Fetch, Update> {
    pub(super) fetch: Fetch,
    pub(super) ready: bool,
    pub(super) update: Update,
}

#[derive(Clone, Debug)]
pub enum Response<T> {
    Fetching,
    Ok(T),
    NotFound,
}

/// Unwrap the value or go to the 404 page
#[macro_export]
macro_rules! unwrap_response {
    ( $api:expr, $value:expr ) => {{
        match $value {
            $crate::stores::client::Response::Fetching => {
                return Default::default();
            }
            $crate::stores::client::Response::Ok(value) => value,
            $crate::stores::client::Response::NotFound => {
                $api.register_alert(crate::stores::client::Alert {
                    level: ::tracing::Level::WARN,
                    message: "Not Found".into(),
                });
                return Default::default();
            }
        }
    }};
}

#[derive(Clone, Debug, PartialEq)]
pub struct Alert {
    pub level: Level,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Store)]
pub struct ClientStore {
    alerts: Vec<Rc<Alert>>,
    client: Client,
    is_fetching: bool,
}

impl ClientStore {
    /// Returns the remaining alerts.
    #[inline]
    pub fn alerts(&self) -> &[Rc<Alert>] {
        &self.alerts
    }

    /// Dismiss an alert.
    pub fn dismiss_alert(&mut self, index: usize, alert: &Rc<Alert>) {
        if let Some(target) = self.alerts.get(index) {
            if Rc::ptr_eq(target, alert) {
                self.alerts.remove(index);
            }
        }
    }

    /// Register an alert.
    pub fn register_alert(&mut self, alert: Alert) {
        match alert.level {
            Level::TRACE => ::tracing::trace!("{}", &alert.message),
            Level::DEBUG => ::tracing::debug!("{}", &alert.message),
            Level::INFO => ::tracing::info!("{}", &alert.message),
            Level::WARN => ::tracing::warn!("{}", &alert.message),
            Level::ERROR => ::tracing::error!("{}", &alert.message),
        }
        self.alerts.push(Rc::new(alert))
    }
}

#[derive(Clone)]
pub struct ApiStore<Store>
where
    Store: ::yewdux::Store,
{
    pub client: Rc<ClientStore>,
    pub store: Rc<Store>,
    pub dispatch: Dispatch<Store>,
    pub dispatch_client: Dispatch<ClientStore>,
    pub navigator: Navigator,
}

impl<Store> ApiStore<Store>
where
    Store: ::yewdux::Store,
{
    pub(super) fn call<T, Fetch, Fut, Update>(self, request: Request<Fetch, Update>)
    where
        T: fmt::Debug,
        Fetch: 'static + FnOnce(Client) -> Fut,
        Fut: Future<Output = Result<T>>,
        Store: Clone,
        Update: 'static + FnOnce(&mut Store, Option<T>),
    {
        let Self {
            client: client_store,
            store: _,
            dispatch,
            dispatch_client,
            navigator: _,
        } = self;
        let Request {
            fetch,
            ready,
            update,
        } = request;

        // Do nothing if the store is locked
        if !ready || client_store.is_fetching {
            return;
        }

        // Lock the store
        dispatch_client.reduce_mut(|client| {
            client.is_fetching = true;
        });

        // Begin fetching
        spawn_local(async move {
            // Update the store
            let error = match fetch(client_store.client.clone()).await {
                Ok(data) => {
                    dispatch.reduce_mut(|store| update(store, Some(data)));
                    None
                }
                Err(error) => {
                    dispatch.reduce_mut(|store| update(store, None));
                    Some(error)
                }
            };
            dispatch_client.reduce_mut(|client| {
                // Alert an error
                if let Some(error) = error {
                    client.register_alert(Alert {
                        level: Level::ERROR,
                        message: format!("Failed to request: {error}"),
                    });
                }
                // Release the store
                client.is_fetching = false;
            })
        })
    }

    /// Raise an alert and go back to the previous page.
    pub fn register_alert(&self, alert: Alert) {
        let _ = alert;
        self.navigator.back();
    }
}

#[hook]
pub fn use_api<Store>() -> ApiStore<Store>
where
    Store: ::yewdux::Store,
{
    let (client, dispatch_client) = use_store::<ClientStore>();
    let (store, dispatch) = use_store::<Store>();
    let navigator = use_navigator().unwrap();

    ApiStore {
        client,
        store,
        dispatch,
        dispatch_client,
        navigator,
    }
}
