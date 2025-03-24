use std::future::{Ready, ready};

use actix_web::{Error, FromRequest, HttpRequest, Result, dev::Payload, web};
use kube::{Client, Config, config::AuthInfo};

use crate::error::internal_server_error;

pub struct KubernetesClient<U = super::User> {
    pub client: Client,
    pub data: U,
}

impl FromRequest for KubernetesClient {
    type Error = Error;

    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        ready(super::UserGuard::from_request_sync(req).and_then(|user| {
            let config = req
                .app_data::<web::Data<Config>>()
                .ok_or_else(internal_server_error)?;

            Ok(Self {
                client: build_client(config, user.token)?,
                data: user.data,
            })
        }))
    }
}

impl FromRequest for KubernetesClient<Option<super::User>> {
    type Error = Error;

    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        ready(
            super::UserGuard::from_request_sync_opt(req).and_then(|user| {
                let config = req
                    .app_data::<web::Data<Config>>()
                    .ok_or_else(internal_server_error)?;

                Ok(match user {
                    Some(user) => Self {
                        client: build_client(config, user.token)?,
                        data: Some(user.data),
                    },
                    None => Self {
                        client: config
                            .as_ref()
                            .clone()
                            .try_into()
                            .map_err(|_| internal_server_error())?,
                        data: None,
                    },
                })
            }),
        )
    }
}

fn build_client(config: &web::Data<Config>, token: String) -> Result<Client> {
    let mut config = config.as_ref().clone();
    config.auth_info = AuthInfo {
        token: Some(token.into()),
        ..Default::default()
    };
    config.try_into().map_err(|_| internal_server_error())
}
