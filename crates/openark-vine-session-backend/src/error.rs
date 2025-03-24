use std::fmt::Display;

use actix_web::HttpResponse;
#[cfg(feature = "tracing")]
use tracing::warn;

pub fn internal_server_error(error: impl Display) -> HttpResponse {
    #[cfg(feature = "tracing")]
    warn!("Failed to list tables: {error}");
    HttpResponse::InternalServerError().finish()
}
