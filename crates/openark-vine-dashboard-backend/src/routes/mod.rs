mod app;
mod tables;
mod users;

use actix_web::{Scope, web};

pub fn build() -> Scope {
    web::scope("")
        .service(self::app::get)
        .service(web::scope("tables").service(self::tables::get))
        .service(web::scope("users").service(self::users::get))
}
