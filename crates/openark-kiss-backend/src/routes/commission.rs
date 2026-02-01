use actix_web::{HttpResponse, Responder, post, web};
use anyhow::{Result, bail};
use jiff::Timestamp;
use kube::{
    Api, CustomResourceExt,
    api::{Patch, PatchParams},
};
use openark_kiss_api::r#box::{BoxCrd, BoxSpec, BoxState, BoxStatus, request::BoxCommissionQuery};
use serde_json::json;
#[cfg(feature = "tracing")]
use tracing::{Level, instrument, warn};

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip(api, patch_params)))]
#[post("")]
async fn post(
    api: web::Data<Api<BoxCrd>>,
    patch_params: web::Data<PatchParams>,
    web::Json(query): web::Json<BoxCommissionQuery>,
) -> impl Responder {
    match try_handle(api, patch_params, query).await {
        Ok(()) => HttpResponse::Ok().json("Ok"),
        Err(error) => {
            #[cfg(feature = "tracing")]
            warn!("failed to commission a box: {error}");
            HttpResponse::Forbidden().json("Err")
        }
    }
}

async fn try_handle(
    api: web::Data<Api<BoxCrd>>,
    patch_params: web::Data<PatchParams>,
    query: BoxCommissionQuery,
) -> Result<()> {
    let name = query.machine.uuid.to_string();

    match api.get_opt(&name).await? {
        Some(r#box) => {
            let crd = BoxCrd::api_resource();
            let patch = Patch::Merge(json!({
                "apiVersion": crd.api_version,
                "kind": crd.kind,
                "spec": BoxSpec {
                    group: r#box.spec.group,
                    machine: query.machine,
                    power: query.power,
                },
                "status": BoxStatus {
                    access: query.access.try_into()?,
                    state: BoxState::Ready,
                    bind_group: if query.reset {
                        None
                    } else {
                        r#box
                            .status
                            .as_ref()
                            .and_then(|status| status.bind_group.as_ref())
                            .cloned()
                    },
                    last_updated: Timestamp::now(),
                },
            }));
            api.patch(&name, &patch_params, &patch).await?;
            api.patch_status(&name, &patch_params, &patch).await?;
        }
        None => bail!("no such box: {name}"),
    }
    Ok(())
}
