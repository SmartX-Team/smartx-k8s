mod routes;
mod store;

use std::net::SocketAddr;

use actix_web::{
    App, HttpResponse, HttpServer, Responder, get, middleware,
    web::{self, Data},
};
use anyhow::Result;
use clap::Parser;
use openark_core::client::HealthState;
use tracing::{Level, instrument};

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO))]
#[get("ping")]
async fn ping() -> impl Responder {
    HttpResponse::Ok().finish()
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO))]
#[get("health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().json(HealthState::Healthy)
}

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// A base URL
    #[arg(long, env = "BASE_URL", value_name = "URL", default_value = "")]
    base_url: String,

    /// An address to bind the server
    #[arg(
        long,
        env = "BIND_ADDR",
        value_name = "ADDR",
        default_value = "0.0.0.0:8000"
    )]
    bind_addr: SocketAddr,

    #[command(flatten)]
    store: self::store::StoreArgs,
}

async fn try_main(args: Args) -> Result<()> {
    // Initialize data
    let Args {
        mut base_url,
        bind_addr: addr,
        store,
    } = args;

    // Remove trailing
    while base_url.ends_with('/') {
        base_url.pop();
    }

    let client = Data::new(::reqwest::Client::default());
    let store = Data::new(store.build()?);

    // Start web server
    HttpServer::new(move || {
        let app = App::new()
            .app_data(Data::clone(&client))
            .app_data(Data::clone(&store));

        let app = app.service(
            web::scope(&base_url)
                .service(ping)
                .service(health)
                .service(self::routes::build()),
        );

        let app = app.wrap(middleware::NormalizePath::new(
            middleware::TrailingSlash::Trim,
        ));

        #[cfg(feature = "cors-allow-any")]
        let app = {
            let cors = ::actix_cors::Cors::default()
                .allow_any_header()
                .allow_any_method()
                .supports_credentials();

            app.wrap(cors)
        };

        #[cfg(feature = "opentelemetry")]
        let app = {
            use actix_web_opentelemetry::{RequestMetrics, RequestTracing};
            app.wrap(RequestTracing::default())
                .wrap(RequestMetrics::default())
        };
        app
    })
    .bind(addr)
    .unwrap_or_else(|e| panic!("failed to bind to {addr}: {e}"))
    .run()
    .await
    .map_err(Into::into)
}

#[::actix_web::main]
async fn main() {
    let args = Args::parse();

    ::openark_core::init_once();

    #[cfg(feature = "tracing")]
    ::tracing::info!("Welcome to OpenARK Spectrum Backend!");

    try_main(args).await.expect("running a server")
}
