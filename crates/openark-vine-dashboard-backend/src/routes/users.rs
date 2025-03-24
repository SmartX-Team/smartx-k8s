use actix_web::{HttpResponse, Responder, get};
use openark_vine_oauth::OptionalUserGuard;
#[cfg(feature = "tracing")]
use tracing::{Level, instrument};

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
#[get("me")]
pub async fn get(user: OptionalUserGuard) -> impl Responder {
    HttpResponse::Ok().json(user.0.map(|user| user.data))
}
