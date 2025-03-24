use actix_web::{
    HttpResponse, HttpResponseBuilder, Responder,
    cookie::{Cookie, time::Duration},
    get,
    http::header,
    web,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::client::ClientExt;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(get).service(callback);
}

#[get("oauth/oidc")]
async fn get(args: web::Data<super::OpenIDClientArgs>) -> impl Responder {
    HttpResponse::Ok().json(args.get_ref())
}

#[derive(Debug, Serialize, Deserialize)]
struct CallbackQuery {
    code: String,
    // iss: Url,
    // session_state: String,
    state: super::State,
}

#[get("oauth/oidc/callback")]
async fn callback(
    args: web::Data<super::OpenIDClientArgs>,
    client: web::Data<Client>,
    query: web::Query<CallbackQuery>,
) -> impl Responder {
    let web::Query(CallbackQuery {
        code,
        // FIXME: Validate issuer
        // iss,
        // session_state,
        state: super::State {
            nonce: _,
            redirect_url,
        },
    }) = query;

    // Exchange token
    let super::OpenIDClientToken {
        access_token,
        token_type: _,
        expires_in,
        id_token: _,
        refresh_token,
    } = match exchange_token(args, client, &code).await {
        Some(token) => token,
        None => return HttpResponse::InternalServerError().finish(),
    };

    // Redirect to the root
    let mut response = redirect_to(redirect_url.as_str());

    // Store tokens
    response.cookie({
        let mut cookie = Cookie::build(super::cookies::ACCESS_TOKEN, access_token)
            .http_only(false) // Allow client-side authorization
            .max_age(Duration::seconds(expires_in as _))
            .path("/") // Allow gateway mode
            .secure(true); // Enforce HTTPS

        if let Some(domain) = redirect_url.host_str() {
            cookie = cookie.domain(domain);
        }
        cookie.finish()
    });
    if let Some(refresh_token) = refresh_token {
        response.cookie({
            let mut cookie = Cookie::build(super::cookies::REFRESH_TOKEN, refresh_token)
                .http_only(false) // Allow client-side authorization
                .max_age(Duration::seconds(expires_in as _))
                .path("/") // Allow gateway mode
                .secure(true); // Enforce HTTPS

            if let Some(domain) = redirect_url.host_str() {
                cookie = cookie.domain(domain);
            }
            cookie.finish()
        });
    }

    // Redirect
    response.finish()
}

async fn exchange_token(
    args: web::Data<super::OpenIDClientArgs>,
    client: web::Data<Client>,
    code: &str,
) -> Option<super::OpenIDClientToken> {
    // Get OpenID Configuration
    let args = args.get_ref();
    let configs = match client.get_auth_configs(args).await {
        Ok(configs) => configs,
        Err(error) => {
            #[cfg(feature = "tracing")]
            ::tracing::error!("Failed to load OIDC configs: {error}");
            return None;
        }
    };

    // Relay to the OpenID provider
    match client.get_auth_token(args, &configs, &code).await {
        Ok(token) => Some(token),
        Err(error) => {
            #[cfg(feature = "tracing")]
            ::tracing::error!("Failed to get OIDC token: {error}");
            return None;
        }
    }
}

fn redirect_to<T>(url: T) -> HttpResponseBuilder
where
    (header::HeaderName, T): header::TryIntoHeaderPair,
{
    let mut builder = HttpResponse::Found();
    builder.insert_header((header::LOCATION, url));
    builder
}
