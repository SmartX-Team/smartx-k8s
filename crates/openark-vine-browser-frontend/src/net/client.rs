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
use url::Url;
use web_sys::{RequestMode, RequestRedirect};

#[derive(Clone, Debug, PartialEq)]
pub struct Client {
    _private: (),
}

impl Client {
    #[inline]
    pub(super) fn new() -> Self {
        Self { _private: () }
    }
}

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
        let url = if cfg!(debug_assertions) {
            // Mockup the api
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
