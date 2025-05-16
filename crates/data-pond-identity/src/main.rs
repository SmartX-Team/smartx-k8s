use std::net::SocketAddr;

use async_trait::async_trait;
use clap::Parser;
use data_pond_api::identity_server::{Identity, IdentityServer};
use tonic::{Request, Response, Result, transport};
#[cfg(feature = "tracing")]
use tracing::info;

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// An address to bind the server
    #[arg(
        long,
        env = "BIND_ADDR",
        value_name = "ADDR",
        default_value = "0.0.0.0:50051"
    )]
    bind_addr: SocketAddr,

    #[command(flatten)]
    server: Server,
}

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Server {}

#[async_trait]
impl Identity for Server {
    async fn get_plugin_info(
        &self,
        request: Request<::data_pond_api::GetPluginInfoRequest>,
    ) -> Result<
        Response<::data_pond_api::GetPluginInfoResponse>,
    > {todo!()}

    async fn get_plugin_capabilities(
        &self,
        request: Request<::data_pond_api::GetPluginCapabilitiesRequest>,
    ) -> Result<
        Response<::data_pond_api::GetPluginCapabilitiesResponse>,
    > {todo!()}

    async fn probe(
        &self,
        request: Request<::data_pond_api::ProbeRequest>,
    ) -> Result<Response<::data_pond_api::ProbeResponse>> {todo!()}
}

async fn try_main(args: Args) -> Result<(), transport::Error> {
    let Args {
        bind_addr: addr,
        server,
    } = args;

    #[cfg(feature = "tracing")]
    info!("Listening on {addr}");

    transport::Server::builder()
        .add_service(IdentityServer::new(server))
        .serve(addr)
        .await
}

#[::tokio::main]
async fn main() {
    let args = Args::parse();

    ::openark_core::init_once();

    #[cfg(feature = "tracing")]
    ::tracing::info!("Welcome to Data Pond Identity Server!");

    try_main(args).await.expect("running a server")
}
