mod data;
mod global;
mod metadata;

use std::borrow::Cow;

use actix_web::{HttpRequest, Scope, web};

fn parse_path<'a>(
    req: &'a HttpRequest,
    base_url: web::Data<String>,
    prefix: &str,
) -> Option<Cow<'a, str>> {
    let path = req.path();

    debug_assert!(path.starts_with(base_url.as_str()));
    let path = &path[base_url.len()..];

    debug_assert!(path.starts_with(prefix));
    let path = &path[prefix.len()..];

    ::percent_encoding::percent_decode_str(path)
        .decode_utf8()
        .ok()
}

pub fn build() -> Scope {
    web::scope("")
        .service(self::global::get)
        .service(web::scope("data").default_service(web::to(self::data::handle)))
        .service(web::scope("metadata").default_service(web::to(self::metadata::handle)))
}
