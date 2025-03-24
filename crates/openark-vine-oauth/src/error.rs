use strum::{Display, EnumString};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("WWW-Authenticate: Bearer realm={realm:?}, {kind}")]
pub struct Error {
    pub realm: String,
    pub kind: ErrorKind,
}

#[cfg(feature = "actix-web")]
impl From<Error> for ::actix_web::Error {
    fn from(value: Error) -> Self {
        ::actix_web::error::ErrorUnauthorized(value.to_string())
    }
}

#[derive(Debug, Error)]
#[cfg(feature = "error-decode")]
pub enum ParseError {
    #[error("{0}")]
    Others(String),
    #[error(transparent)]
    Regex(::regex::Error),
}

#[cfg(feature = "error-decode")]
impl ::core::str::FromStr for Error {
    type Err = ParseError;

    fn from_str(haystack: &str) -> Result<Self, Self::Err> {
        let re = ::regex::Regex::new(
            r#"^WWW-Authenticate:\s*Bearer\s+realm="([^"]+)",\s*error="([^"]+)",\s*error_description="([^"]+)"$"#,
        ).map_err(ParseError::Regex)?;

        match re.captures(haystack) {
            Some(captures) => {
                let realm = &captures[1];
                let error = &captures[2];
                let error_description = &captures[3];

                let kind = match error {
                    "invalid_request" => ErrorKind::InvalidRequest(
                        error_description
                            .parse()
                            .map_err(|_| {
                                #[cfg(feature = "tracing")]
                                ::tracing::warn!(
                                    "Failed to decode WWW-Authenticate error field: \"error_description\": {haystack}"
                                );
                                ParseError::Others(haystack.into())
                            })?,
                    ),
                    "invalid_token" => ErrorKind::InvalidToken(
                        error_description
                            .parse()
                            .map_err(|_| {
                                #[cfg(feature = "tracing")]
                                ::tracing::warn!(
                                    "Failed to decode WWW-Authenticate error field: \"error_description\": {haystack}"
                                );
                                ParseError::Others(haystack.into())
                            })?,
                    ),
                    _ => {
                        #[cfg(feature = "tracing")]
                        ::tracing::warn!(
                            "Failed to decode WWW-Authenticate error field: \"error\": {haystack}"
                        );
                        return Err(ParseError::Others(haystack.into()));
                    }
                };

                Ok(Self {
                    realm: realm.into(),
                    kind,
                })
            }
            None => {
                #[cfg(feature = "tracing")]
                ::tracing::warn!("Failed to decode WWW-Authenticate error: {haystack}");
                Err(ParseError::Others(haystack.into()))
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum ErrorKind {
    #[error("error=\"invalid_request\", error_description=\"{0}\"")]
    InvalidRequest(ErrorInvalidRequest),
    #[error("error=\"invalid_token\", error_description=\"{0}\"")]
    InvalidToken(ErrorInvalidToken),
}

#[derive(Debug, Display, EnumString)]
pub enum ErrorInvalidRequest {
    AccessTokenMissing,
}

#[derive(Debug, Display, EnumString)]
pub enum ErrorInvalidToken {
    AccessTokenExpired,
    MalformedJwtToken,
}

#[cfg(feature = "actix-web")]
pub(crate) fn internal_server_error() -> ::actix_web::Error {
    ::actix_web::error::ErrorInternalServerError("Internal Server Error")
}
