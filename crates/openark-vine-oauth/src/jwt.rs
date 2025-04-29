#[cfg(feature = "actix-web")]
use chrono::{DateTime, Utc};
#[cfg(feature = "actix-web")]
use jsonwebtoken::{Algorithm, DecodingKey, TokenData, Validation, decode};
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "actix-web")]
use crate::error::{ErrorInvalidToken, ErrorKind};

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub(crate) struct JsonWebTokenClaims {
    pub exp: usize,
    pub iat: usize,
    // pub sub: String,
    pub name: String,
    pub groups: Vec<String>,
    pub preferred_username: String,
    pub email: String,
}

#[cfg(feature = "actix-web")]
impl JsonWebTokenClaims {
    pub(crate) fn decode(token: &str) -> Result<Self, ErrorKind> {
        // FIXME: Validate JWT Claims
        let key = DecodingKey::from_secret(b"");
        let validation = {
            let mut validation = Validation::new(Algorithm::RS256);
            validation.insecure_disable_signature_validation();
            validation.validate_aud = false;
            validation
        };

        decode::<Self>(token, &key, &validation)
            .map(|TokenData { claims, header: _ }| claims)
            .map_err(|e| match e.kind() {
                ::jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                    ErrorKind::InvalidToken(ErrorInvalidToken::AccessTokenExpired)
                }
                _ => {
                    #[cfg(feature = "tracing")]
                    ::tracing::warn!("{e}");
                    ErrorKind::InvalidToken(ErrorInvalidToken::MalformedJwtToken)
                }
            })
    }

    #[inline]
    const fn expired_at(&self) -> Option<DateTime<Utc>> {
        DateTime::from_timestamp(self.exp as _, 0)
    }

    pub(crate) fn is_expired(&self) -> bool {
        self.expired_at().is_none_or(|exp| exp >= Utc::now())
    }
}
