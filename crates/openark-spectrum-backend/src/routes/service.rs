use std::collections::HashMap;

use actix_web::{HttpResponse, Responder, post, web};
use anyhow::{Result, anyhow};
use k8s_openapi::api::discovery::v1::Endpoint;
use openark_spectrum_api::schema::{WeightRequest, WeightResponse};
use prometheus_http_query::Client;
#[cfg(feature = "tracing")]
use tracing::{Level, instrument, warn};

use crate::records::RecordArgs;

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
#[post("")]
async fn post(
    client: web::Data<Client>,
    records: web::Data<RecordArgs>,
    web::Json(args): web::Json<WeightRequest<'static, Endpoint>>,
) -> impl Responder {
    match try_handle(client, records, args).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(error) => {
            #[cfg(feature = "tracing")]
            warn!("failed to collect service weights: {error}");
            HttpResponse::Forbidden().json("Err")
        }
    }
}

async fn try_handle(
    client: web::Data<Client>,
    records: web::Data<RecordArgs>,
    args: WeightRequest<'static, Endpoint>,
) -> Result<WeightResponse> {
    let WeightRequest { metadata, list } = args;

    let name = metadata
        .name
        .as_ref()
        .ok_or_else(|| anyhow!("Empty service name"))?;
    let namespace = metadata
        .namespace
        .as_ref()
        .ok_or_else(|| anyhow!("Empty service namespace"))?;

    // Build a PromQL query
    // TODO: validate record by regex
    let record = metadata
        .labels
        .as_ref()
        .and_then(|map| map.get(&records.label_custom_histogram_record))
        .unwrap_or(&records.default_record_service);
    let query = format!(
        r#"{record}{{
            namespace = {namespace:?},
            service_name = {name:?},
        }}"#
    )
    .replace(&[' ', '\n'], "");

    // Evaluate a PromQL query
    let response = client.query(query).get().await?;

    // Parse vector data
    let (data, _stats) = response.into_inner();
    let data = data
        .into_vector()
        .map_err(|_| anyhow!("Invalid PromQL query data"))?;

    // Build a data map
    let map: HashMap<_, _> = data
        .iter()
        .filter_map(|vector| {
            let pod_name = vector.metric().get("pod")?;
            let sample = vector.sample().value();
            Some((pod_name, sample))
        })
        .collect();

    // Collect samples
    let samples = list
        .as_ref()
        .iter()
        .map(|item| {
            item.target_ref
                .as_ref()
                .filter(|target| target.kind.as_deref() == Some("Pod"))
                .and_then(|target| target.name.as_ref())
                .and_then(|pod_name| map.get(pod_name).copied())
                .map(Into::into)
        })
        .collect();

    Ok(WeightResponse { weights: samples })
}
