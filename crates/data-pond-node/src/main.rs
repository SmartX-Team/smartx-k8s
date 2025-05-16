use std::net::SocketAddr;

use async_trait::async_trait;
use clap::Parser;
use data_pond_api::node_server::{Node, NodeServer};
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
impl Node for Server {
    async fn node_stage_volume(
        &self,
        request: Request<::data_pond_api::NodeStageVolumeRequest>,
    ) -> Result<Response<::data_pond_api::NodeStageVolumeResponse>> {
        todo!()
    }

    async fn node_unstage_volume(
        &self,
        request: Request<::data_pond_api::NodeUnstageVolumeRequest>,
    ) -> Result<Response<::data_pond_api::NodeUnstageVolumeResponse>> {
        todo!()
    }

    async fn node_publish_volume(
        &self,
        request: Request<::data_pond_api::NodePublishVolumeRequest>,
    ) -> Result<Response<::data_pond_api::NodePublishVolumeResponse>> {
        todo!()
    }

    async fn node_unpublish_volume(
        &self,
        request: Request<::data_pond_api::NodeUnpublishVolumeRequest>,
    ) -> Result<Response<::data_pond_api::NodeUnpublishVolumeResponse>> {
        todo!()
    }

    async fn node_get_volume_stats(
        &self,
        request: Request<::data_pond_api::NodeGetVolumeStatsRequest>,
    ) -> Result<Response<::data_pond_api::NodeGetVolumeStatsResponse>> {
        todo!()
    }

    async fn node_expand_volume(
        &self,
        request: Request<::data_pond_api::NodeExpandVolumeRequest>,
    ) -> Result<Response<::data_pond_api::NodeExpandVolumeResponse>> {
        todo!()
    }

    async fn node_get_capabilities(
        &self,
        request: Request<::data_pond_api::NodeGetCapabilitiesRequest>,
    ) -> Result<Response<::data_pond_api::NodeGetCapabilitiesResponse>> {
        todo!()
    }

    async fn node_get_info(
        &self,
        request: Request<::data_pond_api::NodeGetInfoRequest>,
    ) -> Result<Response<::data_pond_api::NodeGetInfoResponse>> {
        todo!()
    }
}

async fn try_main(args: Args) -> Result<(), transport::Error> {
    let Args {
        bind_addr: addr,
        server,
    } = args;

    #[cfg(feature = "tracing")]
    info!("Listening on {addr}");

    transport::Server::builder()
        .add_service(NodeServer::new(server))
        .serve(addr)
        .await
}

#[::tokio::main]
async fn main() {
    let args = Args::parse();

    ::openark_core::init_once();

    #[cfg(feature = "tracing")]
    ::tracing::info!("Welcome to Data Pond Node Server!");

    try_main(args).await.expect("running a server")
}
