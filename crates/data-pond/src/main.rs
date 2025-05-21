mod controller;
mod discovery;
mod identity;
mod node;
mod pond;
mod volume;

use std::{
    collections::{BTreeSet, HashMap},
    net::{IpAddr, SocketAddr},
    path::Path,
    sync::Arc,
};

use anyhow::{Error, Result, anyhow, bail};
use clap::{Parser, ValueEnum};
use data_pond_csi::{
    csi::{
        controller_server::ControllerServer, identity_server::IdentityServer,
        node_server::NodeServer,
    },
    pond::{self as api_pond, pond_server::PondServer},
};
use strum::{Display, EnumString};
use tokio::{fs, net::UnixListener, sync::RwLock, try_join};
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

#[derive(
    Copy, Clone, Debug, Display, EnumString, ValueEnum, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
enum Service {
    Controller,
    Identity,
    Node,
    Pond,
}

#[derive(Debug, Default)]
struct State {
    devices: RwLock<HashMap<String, api_pond::Device>>,
}

impl State {
    async fn discover(&self, server: &Server) -> Result<()> {
        let endpoint = self::discovery::discover_devices(server).await?;
        *self.devices.write().await = endpoint;
        Ok(())
    }
}

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct DefaultSettings {
    #[arg(long, env = "DEFAULT_FS_TYPE", default_value = "ext4")]
    fs_type: String,
}

impl DefaultSettings {
    fn volume_options(&self) -> ::data_pond_csi::pond::VolumeOptions {
        ::data_pond_csi::pond::VolumeOptions {
            fs_type: Some(self.fs_type.clone()),
            mount_flags: Default::default(),
            mount_group: Default::default(),
            mount_shared: false,
        }
    }
}

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Server {
    #[command(flatten)]
    default: DefaultSettings,

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

    #[arg(
        long,
        env = "DATA_POND_IO_SOURCES",
        value_name = "SOURCE",
        value_delimiter = ',',
        num_args = 1..,
        required = true,
    )]
    sources: Vec<String>,

    #[clap(skip)]
    state: Arc<State>,
}

impl Server {
    fn node_topology(&self) -> HashMap<String, String> {
        let mut map = HashMap::default();
        map.insert("kubernetes.io/hostname".into(), self.node_id.clone());
        map
    }
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
    let mut use_pond = false;
    for service in services {
        match service {
            Service::Controller => {
                let server = self::controller::Server::try_new(server.default.clone()).await?;
                routes.add_service(ControllerServer::new(server))
            }
            Service::Identity => routes.add_service(IdentityServer::new(server.clone())),
            Service::Node => routes.add_service(NodeServer::new(server.clone())),
            Service::Pond => {
                use_pond = true;
                continue;
            }
        };
    }
    let router = transport::Server::builder().add_routes(routes.routes());

    #[cfg(feature = "tracing")]
    info!("Listening on {csi_endpoint}");

    // Validate endpoint
    let task_csi = async move {
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

                router.serve(addr).await.map_err(Into::into)
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
                router
                    .serve_with_incoming(incoming)
                    .await
                    .map_err(Into::into)
            }
            scheme => bail!("Unexpected CSI endpoint scheme: {scheme}"),
        }
    };

    if use_pond {
        let task_pond = async move {
            let addr = SocketAddr::new(server.pod_ip, server.pond_port);

            #[cfg(feature = "tracing")]
            info!("Listening pond server on tcp://{addr}");

            server.state.discover(&server).await?;
            transport::Server::builder()
                .add_service(PondServer::new(server.clone()))
                .serve(addr)
                .await
                .map_err(Error::from)
        };

        let ((), ()) = try_join!(task_csi, task_pond)?;
        Ok(())
    } else {
        task_csi.await
    }
}

#[::tokio::main]
async fn main() {
    let args = Args::parse();

    ::openark_core::init_once();

    #[cfg(feature = "tracing")]
    ::tracing::info!("Welcome to Data Pond Node Server!");

    try_main(args).await.expect("running a server")
}
