use actix_web::{HttpResponse, Responder, Scope, get, post, web};
use kube::{Api, Client, ResourceExt, api::ListParams};
use openark_vine_oauth::User;
use openark_vine_session_api::{
    command::{SessionCommandCrd, SessionCommandView},
    exec::ExecArgs,
};
#[cfg(feature = "tracing")]
use tracing::{Level, instrument, warn};

use crate::{LabelArgs, utils::build_label_selector};

pub fn build() -> Scope {
    web::scope("").service(list).service(exec)
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
#[get("commands")]
pub async fn list(
    labels: web::Data<LabelArgs>,
    // Add support for guest users
    kube: web::Data<Client>,
    user: User,
) -> impl Responder {
    let api = Api::<SessionCommandCrd>::default_namespaced(kube.as_ref().clone());
    let lp = ListParams {
        label_selector: Some(build_label_selector(labels, None, &user)),
        ..Default::default()
    };

    match api.list(&lp).await {
        Ok(list) => {
            let mut items = list.items.into_iter().map(convert).collect::<Vec<_>>();
            items.sort_by_key(|sc| (sc.alias.clone(), sc.name.clone()));
            HttpResponse::Ok().json(items)
        }
        Err(error) => {
            #[cfg(feature = "tracing")]
            warn!("Failed to list session commands: {error}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

fn convert(cr: SessionCommandCrd) -> SessionCommandView {
    SessionCommandView {
        alias: None,
        name: cr.name_any(),
        spec: cr.spec,
    }
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
#[post("exec")]
pub async fn exec(
    labels: web::Data<LabelArgs>,
    // Add support for guest users
    kube: web::Data<Client>,
    user: User,
    args: web::Json<ExecArgs>,
) -> impl Responder {
    let web::Json(mut args) = args;
    args.label_selector = Some(build_label_selector(
        labels,
        args.label_selector.as_deref(),
        &user,
    ));

    match ::openark_vine_session_exec::exec(kube.as_ref().clone(), &args).await {
        Ok(session) => {
            session.join().await;
            HttpResponse::Ok().json(())
        }
        Err(error) => {
            #[cfg(feature = "tracing")]
            warn!("Failed to exec: {error}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
