#[cfg(feature = "client")]
pub mod client;
pub mod error;
mod jwt;
#[cfg(feature = "kube")]
mod kube;
#[cfg(feature = "actix-web")]
mod parser;
#[cfg(feature = "actix-web")]
pub mod webhook;

use url::Url;

#[cfg(feature = "kube")]
pub use self::kube::KubernetesClient;
#[cfg(feature = "actix-web")]
pub use self::parser::{OptionalUserGuard, UserGuard};

pub mod cookies {
    pub const ACCESS_TOKEN: &str = "_openark_vine_oauth_access_token";
    pub const REFRESH_TOKEN: &str = "_openark_vine_oauth_refresh_token";
}

const NONCE_SIZE: usize = 16;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "clap", derive(::clap::Parser))]
#[cfg_attr(feature = "clap", command(author, version, about, long_about = None))]
#[cfg_attr(feature = "schemars", derive(::schemars::JsonSchema))]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
pub struct OpenIDClientArgs {
    #[cfg_attr(feature = "clap", arg(long, env = "OAUTH_CONFIG_URL"))]
    #[cfg_attr(feature = "serde", serde(rename = "configUrl"))]
    pub oauth_config_url: Url,

    #[cfg_attr(feature = "clap", arg(long, env = "OAUTH_CLIENT_ID"))]
    #[cfg_attr(feature = "serde", serde(rename = "clientId"))]
    pub oauth_client_id: String,

    #[cfg(feature = "actix-web")]
    #[cfg_attr(feature = "clap", arg(long, env = "OAUTH_CLIENT_ORIGIN"))]
    #[cfg_attr(feature = "serde", serde(skip))]
    pub oauth_client_origin: Option<String>,

    #[cfg(feature = "actix-web")]
    #[cfg_attr(feature = "clap", arg(long, env = "OAUTH_CLIENT_SECRET"))]
    #[cfg_attr(feature = "serde", serde(skip))]
    pub oauth_client_secret: String,

    #[cfg_attr(feature = "clap", arg(long, env = "OAUTH_CLIENT_REDIRECT_URL"))]
    #[cfg_attr(feature = "serde", serde(rename = "redirectUrl"))]
    pub oauth_redirect_url: Url,

    #[cfg_attr(feature = "clap", arg(long, env = "OAUTH_SCOPES"))]
    #[cfg_attr(feature = "serde", serde(rename = "scopes"))]
    pub oauth_scopes: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(::schemars::JsonSchema))]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub struct OpenIDClientToken {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u32,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub id_token: Option<String>,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub refresh_token: Option<String>,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "clap", derive(::clap::Parser))]
#[cfg_attr(feature = "clap", command(author, version, about, long_about = None))]
#[cfg_attr(feature = "schemars", derive(::schemars::JsonSchema))]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub struct OpenIDConfiguration {
    pub authorization_endpoint: Url,
    pub token_endpoint: Url,
}

#[derive(Clone, Debug, PartialEq)]
pub struct State {
    nonce: [u8; NONCE_SIZE],
    redirect_url: Url,
}

impl State {
    /// Generate a new state with given redirect_url and random nonce.
    #[cfg(feature = "rand")]
    pub fn new(redirect_url: Url) -> Self {
        Self {
            nonce: {
                let mut buf = [0u8; NONCE_SIZE];
                ::getrandom::fill(&mut buf).expect("Failed to generate nonce");
                buf
            },
            redirect_url,
        }
    }
}

#[cfg(feature = "serde")]
mod impl_convert_for_state {
    use core::fmt;

    use std::str::FromStr;

    use base64::{Engine, engine};
    use serde::{Deserialize, Serialize};
    use url::Url;

    impl super::State {
        const ENGINE: engine::GeneralPurpose = engine::general_purpose::URL_SAFE;
    }

    impl FromStr for super::State {
        type Err = ::anyhow::Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct State {
                nonce: [u8; super::NONCE_SIZE],
                redirect_url: Url,
            }

            let value = super::State::ENGINE
                .decode(s)
                .map_err(|e| ::anyhow::anyhow!(e))?;
            ::serde_json::from_slice(&value)
                .map(
                    |State {
                         nonce,
                         redirect_url,
                     }| super::State {
                        nonce,
                        redirect_url,
                    },
                )
                .map_err(Into::into)
        }
    }

    impl TryFrom<&super::State> for String {
        type Error = ::serde_json::Error;

        fn try_from(value: &super::State) -> Result<Self, Self::Error> {
            #[derive(Serialize)]
            #[serde(rename_all = "camelCase")]
            struct State<'a> {
                nonce: &'a [u8; super::NONCE_SIZE],
                redirect_url: &'a Url,
            }

            let super::State {
                nonce,
                redirect_url,
            } = value;
            let value = ::serde_json::to_vec(&State {
                nonce,
                redirect_url,
            })?;

            Ok(super::State::ENGINE.encode(value))
        }
    }

    impl fmt::Display for super::State {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            String::try_from(self)
                .or(Err(fmt::Error))
                .and_then(|state| state.fmt(f))
        }
    }
}

#[cfg(feature = "serde")]
mod impl_serde_for_state {
    use std::borrow::Cow;

    #[cfg(feature = "schemars")]
    use schemars::{JsonSchema, Schema, SchemaGenerator};
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de, ser};

    #[cfg_attr(feature = "schemars", derive(::schemars::JsonSchema))]
    #[derive(Serialize, Deserialize)]
    #[serde(transparent)]
    struct State<'a>(Cow<'a, str>);

    impl Serialize for super::State {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            String::try_from(self)
                .map_err(<S::Error as ser::Error>::custom)
                .and_then(|state| State(Cow::Owned(state)).serialize(serializer))
        }
    }

    impl<'de> Deserialize<'de> for super::State {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            <State<'de> as Deserialize>::deserialize(deserializer)
                .and_then(|State(state)| state.parse().map_err(<D::Error as de::Error>::custom))
        }
    }

    #[cfg(feature = "schemars")]
    impl JsonSchema for super::State {
        #[inline]
        fn inline_schema() -> bool {
            <State as JsonSchema>::inline_schema()
        }

        #[inline]
        fn schema_name() -> Cow<'static, str> {
            <State as JsonSchema>::schema_name()
        }

        #[inline]
        fn schema_id() -> Cow<'static, str> {
            <State as JsonSchema>::schema_id()
        }

        #[inline]
        fn json_schema(generator: &mut SchemaGenerator) -> Schema {
            <State as JsonSchema>::json_schema(generator)
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "schemars", derive(::schemars::JsonSchema))]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct User(pub(crate) self::jwt::JsonWebTokenClaims);

impl User {
    #[inline]
    pub fn username(&self) -> &str {
        &self.0.preferred_username
    }
}

pub struct UserClaims {
    pub data: Result<User, self::error::ErrorKind>,
    pub refresh_token: Option<String>,
}

impl UserClaims {
    /// Infer the user from the browser.
    #[cfg(all(target_arch = "wasm32", feature = "client"))]
    pub fn infer() -> Self {
        use web_sys::wasm_bindgen::JsCast;

        // Get HTML Document
        let document = ::web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.dyn_into::<::web_sys::HtmlDocument>().ok())
            .expect("HtmlDocument not found");

        // Decode cookies
        let cookies = document.cookie().ok();
        let jar = cookies.as_deref().map(|s| {
            ::cookie::Cookie::split_parse(s)
                .filter_map(|result| result.ok())
                .collect::<Vec<_>>()
        });

        // Try getting refresh token
        let refresh_token = jar
            .as_ref()
            .and_then(|jar| {
                jar.iter()
                    .find(|&cookie| cookie.name() == self::cookies::REFRESH_TOKEN)
            })
            .map(|cookie| cookie.value().into());

        // Try getting access token
        let access_token = match jar.as_ref().and_then(|jar| {
            jar.iter()
                .find(|&cookie| cookie.name() == self::cookies::ACCESS_TOKEN)
        }) {
            Some(cookie) => cookie,
            None => {
                return Self {
                    data: Err(self::error::ErrorKind::InvalidRequest(
                        self::error::ErrorInvalidRequest::AccessTokenMissing,
                    )),
                    refresh_token,
                };
            }
        };

        // Parse the token
        Self::infer_with(access_token.value(), refresh_token)
    }

    /// Infer the user with the given token.
    #[cfg(feature = "client")]
    pub fn infer_with(access_token: &str, refresh_token: Option<String>) -> Self {
        let mut claims = self::jwt::JsonWebTokenClaims::decode(access_token);
        if claims.as_ref().is_ok_and(|claims| claims.is_expired()) {
            claims = Err(self::error::ErrorKind::InvalidToken(
                self::error::ErrorInvalidToken::AccessTokenExpired,
            ));
        }
        Self {
            data: claims.map(User),
            refresh_token,
        }
    }
}
