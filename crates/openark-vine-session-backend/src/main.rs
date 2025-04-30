mod bindings;
mod commands;
mod utils;

use std::net::SocketAddr;

use actix_web::{
    App, HttpResponse, HttpServer, Responder, get, middleware,
    web::{self, Data},
};
use anyhow::Result;
use clap::Parser;
use kube::{Client, Config};
use openark_core::client::HealthState;
use openark_vine_oauth::OpenIDClientArgs;
use tracing::{Level, instrument};

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
        default_value = "0.0.0.0:9090"
    )]
    bind_addr: SocketAddr,

    /// Whether to enable APIserver
    #[arg(long, env = "ENABLE_APISERVER")]
    enable_apiserver: bool,

    #[command(flatten)]
    labels: LabelArgs,

    /// Default namespace name
    #[arg(long, env = "NAMESPACE", value_name = "NAME")]
    namespace: Option<String>,

    #[command(flatten)]
    openid: OpenIDClientArgs,
}

#[derive(Parser)]
struct LabelArgs {
    #[arg(long, env = "OPENARK_LABEL_BIND")]
    label_bind: String,

    #[arg(long, env = "OPENARK_LABEL_BIND_USER")]
    label_bind_user: String,
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
        enable_apiserver,
        labels,
        namespace,
        openid,
    } = args;

    // Remove trailing
    while base_url.ends_with('/') {
        base_url.pop();
    }

    let apiserver_base_url = Data::new(if enable_apiserver {
        Some(base_url.clone())
    } else {
        None
    });
    let config = {
        let mut config = Config::infer().await?;
        if let Some(namespace) = namespace {
            config.default_namespace = namespace;
        }
        config
    };
    let client = Data::new(Client::try_from(config.clone())?);
    let labels = Data::new(labels);
    let openid = Data::new(openid);
    let reqwest = Data::new(::reqwest::Client::new());

    // Start web server
    HttpServer::new(move || {
        let app = App::new()
            .app_data(Data::clone(&apiserver_base_url))
            .app_data(Data::clone(&client))
            .app_data(Data::clone(&labels))
            .app_data(Data::clone(&openid))
            .app_data(Data::clone(&reqwest));

        let app = app
            .service(
                web::scope(&base_url)
                    .service(ping)
                    .service(health)
                    .service(self::bindings::build()),
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
    ::tracing::info!("Welcome to OpenARK VINE Session Backend!");

    try_main(args).await.expect("running a server")
}
