use actix_web::{HttpResponse, Responder, get, web};
use openark_vine_browser_api::global::{GlobalConfiguration, GlobalConfigurationSpec};
use openark_vine_oauth::OptionalUserGuard;
#[cfg(feature = "tracing")]
use tracing::{Level, instrument};

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
#[get("")]
pub async fn get(
    conf: web::Data<GlobalConfigurationSpec>,
    user: OptionalUserGuard,
) -> impl Responder {
    HttpResponse::Ok().json(GlobalConfiguration {
        spec: conf.get_ref().clone(),
        user: None,
    })
}
