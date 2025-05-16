mod controller;
mod identity;
mod node;

use std::{
    collections::BTreeSet,
    path::Path,
    sync::{Arc, atomic::AtomicBool},
};

use anyhow::{Result, bail};
use clap::{Parser, ValueEnum};
use data_pond_api::{
    controller_server::ControllerServer, identity_server::IdentityServer, node_server::NodeServer,
};
use tokio::{fs, net::UnixListener};
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{service::RoutesBuilder, transport};
#[cfg(feature = "tracing")]
use tracing::info;
use url::Url;

#[macro_export]
macro_rules! todo {
    ( $name:expr ) => {{
        return Err(::tonic::Status::unimplemented(concat!(
            $name,
            "is not implemented yet",
        )));
    }};
}

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(
        long,
        env = "CSI_ENDPOINT",
        value_name = "URL",
        default_value = "unix:///csi/csi.sock"
    )]
    csi_endpoint: Url,

    #[command(flatten)]
    server: Server,

    #[arg(
        long,
        env = "DATA_POND_SERVICES",
        value_name = "SERVICE",
        value_delimiter = ',',
        num_args = 1..,
        required = true,
    )]
    services: Vec<Service>,
}

#[derive(Copy, Clone, Debug, ValueEnum, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Service {
    Controller,
    Identity,
    Node,
}

#[derive(Clone, Debug, Default)]
struct State {
    ready: Arc<AtomicBool>,
}

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Server {
    #[clap(skip)]
    state: State,
}

async fn try_main(args: Args) -> Result<()> {
    let Args {
        csi_endpoint,
        server,
        services,
    } = args;

    let services: BTreeSet<_> = services.into_iter().collect();

    #[cfg(feature = "tracing")]
    info!("Activated Services: {services:?}");

    let mut routes = RoutesBuilder::default();
    for service in services {
        match service {
            Service::Controller => routes.add_service(ControllerServer::new(server.clone())),
            Service::Identity => routes.add_service(IdentityServer::new(server.clone())),
            Service::Node => routes.add_service(NodeServer::new(server.clone())),
        };
    }

    #[cfg(feature = "tracing")]
    info!("Listening on {csi_endpoint}");

    let incoming = {
        // Validate endpoint
        match csi_endpoint.host_str() {
            None => (),
            Some(host) => bail!("Unexpected CSI endpoint host: {host}"),
        }
        match csi_endpoint.scheme() {
            "unix" => (),
            scheme => bail!("Unexpected CSI endpoint scheme: {scheme}"),
        }

        // Remove old socket file
        let path = Path::new(csi_endpoint.path());
        if path.exists() {
            fs::remove_file(path).await?;
        }

        // Bind to a Unix socket listener
        let listener = UnixListener::bind(path)?;
        UnixListenerStream::new(listener)
    };

    transport::Server::builder()
        .add_routes(routes.routes())
        .serve_with_incoming(incoming)
        .await
        .map_err(Into::into)
}

#[::tokio::main]
async fn main() {
    let args = Args::parse();

    ::openark_core::init_once();

    #[cfg(feature = "tracing")]
    ::tracing::info!("Welcome to Data Pond Node Server!");

    try_main(args).await.expect("running a server")
}
