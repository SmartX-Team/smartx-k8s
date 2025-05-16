use std::net::SocketAddr;

use async_trait::async_trait;
use clap::Parser;
use data_pond_api::controller_server::{Controller, ControllerServer};
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
impl Controller for Server {
    async fn create_volume(
        &self,
        request: Request<::data_pond_api::CreateVolumeRequest>,
    ) -> Result<Response<::data_pond_api::CreateVolumeResponse>> {
        todo!()
    }

    async fn delete_volume(
        &self,
        request: Request<::data_pond_api::DeleteVolumeRequest>,
    ) -> Result<Response<::data_pond_api::DeleteVolumeResponse>> {
        todo!()
    }

    async fn controller_publish_volume(
        &self,
        request: Request<::data_pond_api::ControllerPublishVolumeRequest>,
    ) -> Result<Response<::data_pond_api::ControllerPublishVolumeResponse>> {
        todo!()
    }

    async fn controller_unpublish_volume(
        &self,
        request: Request<::data_pond_api::ControllerUnpublishVolumeRequest>,
    ) -> Result<Response<::data_pond_api::ControllerUnpublishVolumeResponse>> {
        todo!()
    }

    async fn validate_volume_capabilities(
        &self,
        request: Request<::data_pond_api::ValidateVolumeCapabilitiesRequest>,
    ) -> Result<Response<::data_pond_api::ValidateVolumeCapabilitiesResponse>> {
        todo!()
    }

    async fn list_volumes(
        &self,
        request: Request<::data_pond_api::ListVolumesRequest>,
    ) -> Result<Response<::data_pond_api::ListVolumesResponse>> {
        todo!()
    }

    async fn get_capacity(
        &self,
        request: Request<::data_pond_api::GetCapacityRequest>,
    ) -> Result<Response<::data_pond_api::GetCapacityResponse>> {
        todo!()
    }

    async fn controller_get_capabilities(
        &self,
        request: Request<::data_pond_api::ControllerGetCapabilitiesRequest>,
    ) -> Result<Response<::data_pond_api::ControllerGetCapabilitiesResponse>> {
        todo!()
    }

    async fn create_snapshot(
        &self,
        request: Request<::data_pond_api::CreateSnapshotRequest>,
    ) -> Result<Response<::data_pond_api::CreateSnapshotResponse>> {
        todo!()
    }

    async fn delete_snapshot(
        &self,
        request: Request<::data_pond_api::DeleteSnapshotRequest>,
    ) -> Result<Response<::data_pond_api::DeleteSnapshotResponse>> {
        todo!()
    }

    async fn list_snapshots(
        &self,
        request: Request<::data_pond_api::ListSnapshotsRequest>,
    ) -> Result<Response<::data_pond_api::ListSnapshotsResponse>> {
        todo!()
    }

    async fn get_snapshot(
        &self,
        request: Request<::data_pond_api::GetSnapshotRequest>,
    ) -> Result<Response<::data_pond_api::GetSnapshotResponse>> {
        todo!()
    }

    async fn controller_expand_volume(
        &self,
        request: Request<::data_pond_api::ControllerExpandVolumeRequest>,
    ) -> Result<Response<::data_pond_api::ControllerExpandVolumeResponse>> {
        todo!()
    }

    async fn controller_get_volume(
        &self,
        request: Request<::data_pond_api::ControllerGetVolumeRequest>,
    ) -> Result<Response<::data_pond_api::ControllerGetVolumeResponse>> {
        todo!()
    }

    async fn controller_modify_volume(
        &self,
        request: Request<::data_pond_api::ControllerModifyVolumeRequest>,
    ) -> Result<Response<::data_pond_api::ControllerModifyVolumeResponse>> {
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
        .add_service(ControllerServer::new(server))
        .serve(addr)
        .await
}

#[::tokio::main]
async fn main() {
    let args = Args::parse();

    ::openark_core::init_once();

    #[cfg(feature = "tracing")]
    ::tracing::info!("Welcome to Data Pond Controller Server!");

    try_main(args).await.expect("running a server")
}
