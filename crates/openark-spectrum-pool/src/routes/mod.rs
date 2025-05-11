mod service;

use actix_web::{Scope, web};

pub fn build() -> Scope {
    web::scope("").service(
        web::scope("v1/Service")
            .service(self::service::post)
            .service(self::service::post_commit),
    )
}
