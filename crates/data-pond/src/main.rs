mod controller;
mod discovery;
mod identity;
mod node;
mod pond;

use std::{
    collections::{BTreeMap, BTreeSet},
    net::IpAddr,
    path::Path,
    sync::Arc,
};

use anyhow::{Result, anyhow, bail};
use clap::{Parser, ValueEnum};
use data_pond_api::{
    csi::{
        self, controller_server::ControllerServer, identity_server::IdentityServer,
        node_server::NodeServer,
    },
    pond::{self as api_pond, pond_server::PondServer},
};
use tokio::{fs, net::UnixListener, sync::RwLock};
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
            " is not implemented yet",
        )));
    }};
}

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, env = "CSI_ENDPOINT", value_name = "URL")]
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
    Pond,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct EndpointName(String);

#[derive(Debug)]
struct Endpoint {
    devices: Vec<api_pond::Device>,
}

#[derive(Debug)]
struct Volume {
    data: csi::Volume,
    condition: csi::VolumeCondition,
    published_node_ids: BTreeSet<String>,
}

#[derive(Debug, Default)]
struct State {
    endpoints: RwLock<BTreeMap<EndpointName, Endpoint>>,
    volumes: RwLock<BTreeMap<String, Volume>>,
}

impl State {
    async fn discover(&self, server: &Server) -> Result<()> {
        let name = EndpointName(format!("tcp://{}:{}", server.pod_ip, server.pond_port));
        let endpoint = self::discovery::discover(server).await?;
        self.endpoints.write().await.insert(name, endpoint);
        Ok(())
    }
}

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Server {
    #[arg(long, env = "DRIVER_NAME")]
    driver_name: String,

    #[arg(long, env = "NODE_ID")]
    node_id: String,

    #[arg(long, env = "POD_IP")]
    pod_ip: IpAddr,

    #[arg(
        long,
        env = "DATA_POND_PORT",
        value_name = "PORT",
        default_value_t = 9090
    )]
    pond_port: u16,

    #[clap(skip)]
    state: Arc<State>,
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
            Service::Pond => {
                server.state.discover(&server).await?;
                routes.add_service(PondServer::new(server.clone()))
            }
        };
    }
    let router = transport::Server::builder().add_routes(routes.routes());

    #[cfg(feature = "tracing")]
    info!("Listening on {csi_endpoint}");

    // Validate endpoint
    match csi_endpoint.scheme() {
        "tcp" => {
            let host = csi_endpoint
                .host_str()
                .ok_or_else(|| anyhow!("Missing CSI endpoint host"))?;
            let port = csi_endpoint
                .port()
                .ok_or_else(|| anyhow!("Missing CSI endpoint port"))?;

            let addr = format!("{host}:{port}")
                .parse()
                .map_err(|error| anyhow!("Failed to parse CSI endpoint address: {error}"))?;

            router.serve(addr).await
        }
        "unix" => {
            match csi_endpoint.host_str() {
                None => (),
                Some(host) => bail!("Unexpected CSI endpoint host: {host}"),
            }

            // Remove old socket file
            let path = Path::new(csi_endpoint.path());
            if path.exists() {
                fs::remove_file(path).await?;
            }

            // Bind to a Unix socket listener
            let listener = UnixListener::bind(path)?;
            let incoming = UnixListenerStream::new(listener);
            router.serve_with_incoming(incoming).await
        }
        scheme => bail!("Unexpected CSI endpoint scheme: {scheme}"),
    }
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
