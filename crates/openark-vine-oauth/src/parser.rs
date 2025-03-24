use std::future::{Ready, ready};

use actix_web::{FromRequest, HttpRequest, dev::Payload, http::header, web};

use crate::{
    error::{Error, ErrorInvalidRequest, ErrorKind, internal_server_error},
    jwt::JsonWebTokenClaims,
};

fn get_token(req: &HttpRequest) -> Option<String> {
    const AUTHORIZATION_HEADER_PREFIX: &'static str = "Bearer ";

    req.cookie(super::cookies::ACCESS_TOKEN)
        .map(|c| c.value().into())
        .or_else(|| {
            req.headers()
                .get(header::AUTHORIZATION)
                .and_then(|header| header.to_str().ok())
                .filter(|&header| header.starts_with(AUTHORIZATION_HEADER_PREFIX))
                .map(|header| header[AUTHORIZATION_HEADER_PREFIX.len()..].into())
        })
}

fn get_args(req: &HttpRequest) -> Result<&web::Data<super::OpenIDClientArgs>, ::actix_web::Error> {
    req.app_data().ok_or_else(internal_server_error)
}

#[inline]
fn convert_error(args: &web::Data<super::OpenIDClientArgs>, kind: ErrorKind) -> ::actix_web::Error {
    From::from(Error {
        realm: args.oauth_client_id.clone(),
        kind,
    })
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "schemars", derive(::schemars::JsonSchema))]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct UserGuard {
    pub data: super::User,
    pub token: String,
}

impl UserGuard {
    pub(crate) fn from_request_sync(req: &HttpRequest) -> Result<Self, ::actix_web::Error> {
        get_args(req).and_then(|args| {
            UserGuard::from_request_sync_with_args(req).map_err(|kind| convert_error(args, kind))
        })
    }

    fn from_request_sync_with_args(req: &HttpRequest) -> Result<Self, ErrorKind> {
        match Self::from_request_sync_opt_with_args(req)? {
            Some(guard) => Ok(guard),
            None => Err(ErrorKind::InvalidRequest(
                ErrorInvalidRequest::AccessTokenMissing,
            )),
        }
    }

    pub(crate) fn from_request_sync_opt(
        req: &HttpRequest,
    ) -> Result<Option<Self>, ::actix_web::Error> {
        get_args(req).and_then(|args| {
            UserGuard::from_request_sync_opt_with_args(req)
                .map_err(|kind| convert_error(args, kind))
        })
    }

    fn from_request_sync_opt_with_args(req: &HttpRequest) -> Result<Option<Self>, ErrorKind> {
        get_token(req)
            .map(|token| {
                JsonWebTokenClaims::decode(&token).map(|claims| UserGuard {
                    data: super::User(claims),
                    token,
                })
            })
            .transpose()
    }
}

impl FromRequest for UserGuard {
    type Error = ::actix_web::Error;

    type Future = Ready<Result<Self, Self::Error>>;

    #[inline]
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        ready(UserGuard::from_request_sync(req))
    }
}

pub struct OptionalUserGuard(pub Option<UserGuard>);

impl FromRequest for OptionalUserGuard {
    type Error = ::actix_web::Error;

    type Future = Ready<Result<Self, Self::Error>>;

    #[inline]
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        ready(UserGuard::from_request_sync_opt(req).map(Self))
    }
}

impl FromRequest for super::User {
    type Error = ::actix_web::Error;

    type Future = Ready<Result<Self, Self::Error>>;

    #[inline]
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        ready(UserGuard::from_request_sync(req).map(|guard| guard.data))
    }
}
