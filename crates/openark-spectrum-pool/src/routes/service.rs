use actix_web::{HttpResponse, Responder, post, web};
use anyhow::Result;
use k8s_openapi::api::discovery::v1::Endpoint;
use openark_spectrum_api::schema::{WeightResponse, WeightedItems};
use reqwest::Client;
#[cfg(feature = "tracing")]
use tracing::{Level, instrument, warn};

use crate::store::Store;

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
#[post("")]
async fn post(
    client: web::Data<Client>,
    store: web::Data<Store>,
    web::Json(args): web::Json<WeightedItems<Endpoint>>,
) -> impl Responder {
    match try_handle(client, store, args).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(error) => {
            #[cfg(feature = "tracing")]
            warn!("failed to collect service binding states: {error}");
            HttpResponse::Forbidden().json("Err")
        }
    }
}

async fn try_handle(
    client: web::Data<Client>,
    store: web::Data<Store>,
    args: WeightedItems<Endpoint>,
) -> Result<WeightResponse> {
    let WeightedItems { items, weights } = args;

    todo!()
}
