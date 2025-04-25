mod commission;
mod new;

use actix_web::{Scope, web};

pub fn build() -> Scope {
    web::scope("")
        .service(web::scope("commission").service(self::commission::post))
        .service(web::scope("new").service(self::new::get))
}
