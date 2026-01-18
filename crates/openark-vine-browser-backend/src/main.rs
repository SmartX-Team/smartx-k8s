mod routes;

use std::{net::SocketAddr, path::PathBuf};

use actix_web::{
    App, HttpResponse, HttpServer, Responder, get, middleware,
    web::{self, Data},
};
use anyhow::Result;
use clap::Parser;
use openark_core::client::HealthState;
use openark_vine_browser_api::global::GlobalConfigurationSpec;
use openark_vine_oauth::OpenIDClientArgs;
use tracing::{Level, instrument};
use url::Url;

#[derive(Parser)]
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
        default_value = "0.0.0.0:8888"
    )]
    bind_addr: SocketAddr,

    #[command(flatten)]
    conf: Configuration,

    /// A data directory
    #[arg(long, env = "DATA_DIR", value_name = "PATH", default_value = ".")]
    data_dir: PathBuf,

    #[command(flatten)]
    openid: OpenIDClientArgs,
}

#[derive(Parser)]
struct Configuration {
    /// An app logo URL
    #[arg(long, env = "APP_LOGO_URL", value_name = "URL")]
    app_logo_url: Option<Url>,

    /// An app redirect URL
    #[arg(long, env = "APP_REDIRECT_URL", value_name = "URL")]
    app_redirect_url: Option<Url>,

    /// An app title
    #[arg(
        long,
        env = "APP_TITLE",
        value_name = "NAME",
        default_value = "OpenARK VINE Browser"
    )]
    app_title: String,
}

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

async fn try_main(args: Args) -> Result<()> {
    // Initialize data
    let Args {
        mut base_url,
        bind_addr: addr,
        conf,
        data_dir,
        openid,
    } = args;

    // Remove trailing
    while base_url.ends_with('/') {
        base_url.pop();
    }

    let base_url = Data::new(base_url);
    let conf = Data::new(GlobalConfigurationSpec {
        title: conf.app_title,
        logo_url: conf.app_logo_url,
        redirect_url: conf.app_redirect_url,
    });
    let data_dir = Data::new(data_dir);
    let openid = Data::new(openid);

    // Start web server
    HttpServer::new(move || {
        let app = App::new()
            .app_data(Data::clone(&base_url))
            .app_data(Data::clone(&conf))
            .app_data(Data::clone(&data_dir))
            .app_data(Data::clone(&openid));

        let app = app
            .service(
                web::scope(&base_url)
                    .service(ping)
                    .service(health)
                    .service(self::routes::build()),
            )
            .configure(::openark_vine_oauth::webhook::config);

        let app = app.wrap(middleware::NormalizePath::new(
            middleware::TrailingSlash::Trim,
        ));

        #[cfg(feature = "cors-allow-any")]
        let app = {
            let mut cors = ::actix_cors::Cors::default()
                .allow_any_header()
                .allow_any_method()
                .block_on_origin_mismatch(true)
                .supports_credentials();

            if let Some(origin) = openid.oauth_client_origin.as_deref() {
                cors = cors.allowed_origin(origin);
            }

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
    ::tracing::info!("Welcome to OpenARK VINE Browser Backend!");

    try_main(args).await.expect("running a server")
}
