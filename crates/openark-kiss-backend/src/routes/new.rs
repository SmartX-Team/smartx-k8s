use actix_web::{HttpResponse, Responder, get, web};
use anyhow::Result;
use jiff::Timestamp;
use kube::{
    Api, CustomResourceExt,
    api::{ObjectMeta, Patch, PatchParams, PostParams},
};
use openark_kiss_api::r#box::{
    BoxAccessSpec, BoxCrd, BoxSpec, BoxState, BoxStatus, request::BoxNewQuery,
};
use serde_json::json;
#[cfg(feature = "tracing")]
use tracing::{Level, instrument, warn};

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip(api, patch_params)))]
#[get("")]
async fn get(
    api: web::Data<Api<BoxCrd>>,
    patch_params: web::Data<PatchParams>,
    web::Query(query): web::Query<BoxNewQuery>,
) -> impl Responder {
    match try_handle(api, patch_params, query).await {
        Ok(()) => HttpResponse::Ok().json("Ok"),
        Err(error) => {
            #[cfg(feature = "tracing")]
            warn!("failed to register a box: {error}");
            HttpResponse::Forbidden().json("Err")
        }
    }
}

async fn try_handle(
    api: web::Data<Api<BoxCrd>>,
    patch_params: web::Data<PatchParams>,
    query: BoxNewQuery,
) -> Result<()> {
    let name = query.machine.uuid.to_string();

    match api.get_opt(&name).await? {
        Some(r#box) => {
            let crd = BoxCrd::api_resource();
            let patch = Patch::Merge(json!({
                "apiVersion": crd.api_version,
                "kind": crd.kind,
                "status": BoxStatus {
                    access: BoxAccessSpec {
                        primary: Some(query.access_primary.try_into()?),
                    },
                    state: BoxState::New,
                    bind_group: r#box.status.as_ref().and_then(|status| status.bind_group.as_ref()).cloned(),
                    last_updated: Timestamp::now(),
                },
            }));
            api.patch_status(&name, &patch_params, &patch).await?;
        }
        None => {
            let data = BoxCrd {
                metadata: ObjectMeta {
                    name: Some(name.clone()),
                    ..Default::default()
                },
                spec: BoxSpec {
                    group: Default::default(),
                    machine: query.machine,
                    power: None,
                },
                status: None,
            };
            let pp = PostParams {
                dry_run: false,
                field_manager: Some("kiss-gateway".into()),
            };
            api.create(&pp, &data).await?;

            let crd = BoxCrd::api_resource();
            let patch = Patch::Merge(json!({
                "apiVersion": crd.api_version,
                "kind": crd.kind,
                "status": BoxStatus {
                    access: BoxAccessSpec {
                        primary: Some(query.access_primary.try_into()?),
                    },
                    state: BoxState::New,
                    bind_group: None,
                    last_updated: Timestamp::now(),
                },
            }));
            api.patch_status(&name, &patch_params, &patch).await?;
        }
    }
    Ok(())
}
