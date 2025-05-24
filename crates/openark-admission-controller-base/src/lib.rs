use std::{
    fs::File,
    io::BufReader,
    net::SocketAddr,
    path::{Path, PathBuf},
};

use actix_web::{
    App, HttpResponse, HttpServer, Responder, get,
    http::Method,
    middleware,
    web::{Data, Json, route},
};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use clap::Parser;
use k8s_openapi::serde::{Serialize, de::DeserializeOwned};
use kube::{
    Resource,
    core::admission::{AdmissionRequest, AdmissionResponse, AdmissionReview},
};
use openark_core::client::HealthState;
use rustls::{ServerConfig, pki_types::PrivateKeyDer};
#[cfg(feature = "tracing")]
use tracing::{Level, instrument};

#[async_trait]
pub trait AdmissionController
where
    Self: Send + Sync,
{
    type Object: Serialize + DeserializeOwned + Resource;

    async fn handle(&self, request: AdmissionRequest<Self::Object>) -> Result<AdmissionResponse>;
}

#[async_trait]
pub trait AdmissionControllerBuilder
where
    Self: AdmissionController,
{
    type Args: ::clap::Args;

    async fn build(args: Self::Args) -> Result<Self>
    where
        Self: Sized;
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args<T>
where
    T: ::clap::Args,
{
    /// An address to bind the server
    #[arg(
        long,
        env = "BIND_ADDR",
        value_name = "ADDR",
        default_value = "0.0.0.0:8443"
    )]
    bind_addr: SocketAddr,

    #[command(flatten)]
    service: T,

    /// A path to the TLS certification file
    #[arg(long, env = "TLS_CERT_PATH", value_name = "PATH")]
    tls_cert: PathBuf,

    /// A path to the TLS key file
    #[arg(long, env = "TLS_KEY_PATH", value_name = "PATH")]
    tls_key: PathBuf,
}

#[cfg_attr(feature = "tracing", instrument(
    level = Level::INFO,
    skip_all,
    fields(
        apiVersion = review.0.request.as_ref().map(|request| &request.types.api_version),
        kind = review.0.request.as_ref().map(|request| &request.types.kind),
        name = review.0.request.as_ref().map(|request| &request.name),
        namespace = review.0.request.as_ref().and_then(|request| request.namespace.as_ref()),
    ),
))]
async fn index<T>(controller: Data<T>, review: Json<AdmissionReview<T::Object>>) -> impl Responder
where
    T: AdmissionController,
{
    let AdmissionReview {
        types,
        request,
        response: _,
    } = review.0;

    let request = match request {
        Some(request) => request,
        None => {
            return HttpResponse::BadRequest().json(AdmissionReview::<T::Object> {
                types,
                request: None,
                response: Some(AdmissionResponse::invalid("Empty request")),
            });
        }
    };

    let response = match controller.handle(request).await {
        Ok(response) => response,
        Err(error) => {
            return HttpResponse::InternalServerError().json(AdmissionReview::<T::Object> {
                types,
                request: None,
                response: Some(AdmissionResponse::invalid(format!(
                    "An error encountered while reviewing: {error}"
                ))),
            });
        }
    };

    HttpResponse::Ok().json(AdmissionReview::<T::Object> {
        types,
        request: None,
        response: Some(response),
    })
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO))]
#[get("/ping")]
async fn ping() -> impl Responder {
    HttpResponse::Ok().finish()
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO))]
#[get("/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().json(HealthState::Healthy)
}

fn load_tls_config(certs_path: &Path, key_path: &Path) -> Result<ServerConfig> {
    // Load TLS key and cert files
    let mut certs_file = BufReader::new(File::open(certs_path)?);
    let mut key_file = BufReader::new(File::open(key_path)?);

    // load TLS certs and key
    // to create a self-signed temporary cert for testing:
    // `openssl req -x509 -newkey rsa:4096 -nodes -keyout key.pem -out cert.pem -days 365 -subj '/CN=localhost'`
    let cert_chain = ::rustls_pemfile::certs(&mut certs_file).collect::<Result<Vec<_>, _>>()?;
    let key_der = ::rustls_pemfile::rsa_private_keys(&mut key_file)
        .next()
        .ok_or_else(|| anyhow!("Cannot find TLS key"))?
        .map(PrivateKeyDer::Pkcs1)?;

    ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key_der)
        .map_err(Into::into)
}

async fn try_loop_forever<T>(args: Args<T::Args>) -> Result<()>
where
    T: 'static + Send + Sync + AdmissionControllerBuilder,
{
    let Args {
        bind_addr: addr,
        service,
        tls_cert,
        tls_key,
    } = args;

    // Initialize service
    let service = Data::new(T::build(service).await?);

    // Initialize TLS
    let tls_config = load_tls_config(&tls_cert, &tls_key)?;

    // Start web server
    HttpServer::new(move || {
        let app = App::new().app_data(Data::clone(&service));

        let app = app
            .route("", route().method(Method::POST).to(index::<T>))
            .service(ping)
            .service(health);

        let app = app.wrap(middleware::NormalizePath::new(
            middleware::TrailingSlash::Trim,
        ));

        #[cfg(feature = "opentelemetry")]
        let app = {
            use actix_web_opentelemetry::{RequestMetrics, RequestTracing};
            app.wrap(RequestTracing::default())
                .wrap(RequestMetrics::default())
        };
        app
    })
    .bind_rustls_0_23(addr, tls_config)
    .unwrap_or_else(|e| panic!("failed to bind to {addr}: {e}"))
    .run()
    .await
    .map_err(Into::into)
}

#[::actix_web::main]
pub async fn loop_forever<T>()
where
    T: 'static + Send + Sync + AdmissionControllerBuilder,
{
    let args = Args::<T::Args>::parse();

    ::openark_core::init_once();

    #[cfg(feature = "tracing")]
    ::tracing::info!("Welcome to OpenARK Admission Controller!");

    try_loop_forever::<T>(args).await.expect("running a server")
}
