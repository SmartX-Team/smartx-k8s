use std::{
    fs::File,
    io::BufReader,
    net::SocketAddr,
    path::{Path, PathBuf},
    process::Stdio,
};

use actix_web::{
    App, HttpResponse, HttpServer, Responder, get, middleware, post,
    web::{Data, Json},
};
use anyhow::{Error, Result, anyhow};
use clap::Parser;
use kube::{
    api::DynamicObject,
    core::admission::{AdmissionRequest, AdmissionResponse},
};
use openark_admission_openapi::AdmissionResult;
use rustls::{ServerConfig, pki_types::PrivateKeyDer};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
    try_join,
};
#[cfg(feature = "tracing")]
use tracing::{Level, instrument};

type AdmissionReview = ::kube::core::admission::AdmissionReview<DynamicObject>;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// An address to bind the server
    #[arg(
        long,
        env = "BIND_ADDR",
        value_name = "ADDR",
        default_value = "0.0.0.0:8443"
    )]
    bind_addr: SocketAddr,

    /// A path to the reviewer script
    #[arg(long, env = "SCRIPT_PATH", value_name = "PATH")]
    script_path: PathBuf,

    /// A path to the TLS certification file
    #[arg(long, env = "TLS_CERT_PATH", value_name = "PATH")]
    tls_cert: PathBuf,

    /// A path to the TLS key file
    #[arg(long, env = "TLS_KEY_PATH", value_name = "PATH")]
    tls_key: PathBuf,
}

struct Script {
    path: PathBuf,
}

impl Script {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }

    async fn run(&self, request: &AdmissionRequest<DynamicObject>) -> Result<AdmissionResponse> {
        let request_data = ::serde_json::to_vec(request)?;
        let capacity = request_data.len();

        let child = Command::new(&self.path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        let mut stdin = child.stdin.unwrap();
        let task_tx = async move {
            stdin.write_all(&request_data).await?;
            stdin.flush().await.map_err(Error::from)
        };

        let mut stdout = child.stdout.unwrap();
        let task_rx = async move {
            let mut buf = Vec::with_capacity(capacity);
            stdout.read_to_end(&mut buf).await?;
            ::serde_json::from_slice(&buf).map_err(Error::from)
        };

        let response = AdmissionResponse::from(request);
        let (_, result) = try_join!(task_tx, task_rx)?;
        Ok(match result {
            AdmissionResult::Deny { message } => response.deny(message),
            AdmissionResult::Pass => response,
            AdmissionResult::Patch { operations: patch } => response.with_patch(patch)?,
        })
    }
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
#[post("/")]
async fn index(script: Data<Script>, review: Json<AdmissionReview>) -> impl Responder {
    let AdmissionReview {
        types,
        request,
        response: _,
    } = review.0;

    let request = match request {
        Some(request) => request,
        None => {
            return HttpResponse::BadRequest().json(AdmissionReview {
                types,
                request: None,
                response: Some(AdmissionResponse::invalid("Empty request")),
            });
        }
    };

    let response = match script.run(&request).await {
        Ok(response) => response,
        Err(error) => {
            return HttpResponse::InternalServerError().json(AdmissionReview {
                types,
                request: None,
                response: Some(AdmissionResponse::invalid(format!(
                    "An error encountered while reviewing: {error}"
                ))),
            });
        }
    };

    HttpResponse::Ok().json(AdmissionReview {
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
    HttpResponse::Ok().json("healthy")
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

async fn try_main(args: Args) -> Result<()> {
    // Initialize data
    let addr = args.bind_addr;
    let script = Data::new(Script::new(args.script_path));

    // Initialize TLS
    let tls_config = load_tls_config(&args.tls_cert, &args.tls_key)?;

    // Start web server
    HttpServer::new(move || {
        let app = App::new().app_data(Data::clone(&script));

        let app = app.service(index).service(ping).service(health);

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
async fn main() {
    let args = Args::parse();

    ::openark_core::init_once();

    #[cfg(feature = "tracing")]
    ::tracing::info!("Welcome to OpenARK Admission Controller!");

    try_main(args).await.expect("running a server")
}
