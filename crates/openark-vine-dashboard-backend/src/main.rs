mod routes;

use std::net::SocketAddr;

use actix_web::{
    App, HttpResponse, HttpServer, Responder, get, middleware,
    web::{self, Data},
};
use anyhow::Result;
use clap::Parser;
use kube::Config;
use openark_vine_dashboard_api::app::AppMetadata;
use openark_vine_oauth::OpenIDClientArgs;
use tracing::{Level, instrument};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// An app name
    #[arg(
        long,
        env = "APP_NAME",
        value_name = "NAME",
        default_value = "openark-vine-dashboard"
    )]
    app_name: String,

    /// An app title
    #[arg(
        long,
        env = "APP_TITLE",
        value_name = "NAME",
        default_value = "OpenARK VINE Dashboard"
    )]
    app_title: String,

    /// An app description
    #[arg(long, env = "APP_DESCRIPTION", value_name = "TEXT")]
    app_description: Option<String>,

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
    labels: LabelArgs,

    /// Default namespace name
    #[arg(long, env = "NAMESPACE", value_name = "ADDR")]
    namespace: Option<String>,

    #[command(flatten)]
    openid: OpenIDClientArgs,
}

#[derive(Parser)]
struct LabelArgs {
    #[arg(long, env = "OPENARK_LABEL_CATEGORY")]
    label_category: String,

    #[arg(long, env = "OPENARK_LABEL_DESCRIPTION")]
    label_description: String,

    #[arg(long, env = "OPENARK_LABEL_TITLE")]
    label_title: String,
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO))]
#[get("ping")]
async fn ping() -> impl Responder {
    HttpResponse::Ok().finish()
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO))]
#[get("health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().json("healthy")
}

async fn try_main(args: Args) -> Result<()> {
    // Initialize data
    let Args {
        app_name: name,
        app_title: title,
        app_description: description,
        mut base_url,
        bind_addr: addr,
        labels,
        namespace,
        openid,
    } = args;

    let app = Data::new(AppMetadata {
        name,
        title: Some(title),
        description,
    });
    let config = Data::new({
        let mut config = Config::infer().await?;
        if let Some(namespace) = namespace {
            config.default_namespace = namespace;
        }
        config
    });
    let labels = Data::new(labels);
    let openid = Data::new(openid);
    let reqwest = Data::new(::reqwest::Client::new());

    // Remove trailing
    while base_url.ends_with('/') {
        base_url.pop();
    }

    // Start web server
    HttpServer::new(move || {
        let app = App::new()
            .app_data(Data::clone(&app))
            .app_data(Data::clone(&config))
            .app_data(Data::clone(&labels))
            .app_data(Data::clone(&openid))
            .app_data(Data::clone(&reqwest));

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
    ::tracing::info!("Welcome to OpenARK VINE Dashboard Backend!");

    try_main(args).await.expect("running a server")
}
