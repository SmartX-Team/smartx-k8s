use async_trait::async_trait;
use data_pond_api::controller_server::Controller;
use tonic::{Request, Response, Result};

#[async_trait]
impl Controller for super::Server {
    async fn create_volume(
        &self,
        request: Request<::data_pond_api::CreateVolumeRequest>,
    ) -> Result<Response<::data_pond_api::CreateVolumeResponse>> {
        dbg!(request.into_inner());
        crate::todo!("create_volume")
    }

    async fn delete_volume(
        &self,
        request: Request<::data_pond_api::DeleteVolumeRequest>,
    ) -> Result<Response<::data_pond_api::DeleteVolumeResponse>> {
        dbg!(request.into_inner());
        crate::todo!("delete_volume")
    }

    async fn controller_publish_volume(
        &self,
        request: Request<::data_pond_api::ControllerPublishVolumeRequest>,
    ) -> Result<Response<::data_pond_api::ControllerPublishVolumeResponse>> {
        dbg!(request.into_inner());
        crate::todo!("controller_publish_volume")
    }

    async fn controller_unpublish_volume(
        &self,
        request: Request<::data_pond_api::ControllerUnpublishVolumeRequest>,
    ) -> Result<Response<::data_pond_api::ControllerUnpublishVolumeResponse>> {
        dbg!(request.into_inner());
        crate::todo!("controller_unpublish_volume")
    }

    async fn validate_volume_capabilities(
        &self,
        request: Request<::data_pond_api::ValidateVolumeCapabilitiesRequest>,
    ) -> Result<Response<::data_pond_api::ValidateVolumeCapabilitiesResponse>> {
        dbg!(request.into_inner());
        crate::todo!("validate_volume_capabilities")
    }

    async fn list_volumes(
        &self,
        request: Request<::data_pond_api::ListVolumesRequest>,
    ) -> Result<Response<::data_pond_api::ListVolumesResponse>> {
        dbg!(request.into_inner());
        crate::todo!("list_volumes")
    }

    async fn get_capacity(
        &self,
        request: Request<::data_pond_api::GetCapacityRequest>,
    ) -> Result<Response<::data_pond_api::GetCapacityResponse>> {
        dbg!(request.into_inner());
        crate::todo!("get_capacity")
    }

    async fn controller_get_capabilities(
        &self,
        request: Request<::data_pond_api::ControllerGetCapabilitiesRequest>,
    ) -> Result<Response<::data_pond_api::ControllerGetCapabilitiesResponse>> {
        dbg!(request.into_inner());
        crate::todo!("controller_get_capabilities")
    }

    async fn create_snapshot(
        &self,
        request: Request<::data_pond_api::CreateSnapshotRequest>,
    ) -> Result<Response<::data_pond_api::CreateSnapshotResponse>> {
        dbg!(request.into_inner());
        crate::todo!("create_snapshot")
    }

    async fn delete_snapshot(
        &self,
        request: Request<::data_pond_api::DeleteSnapshotRequest>,
    ) -> Result<Response<::data_pond_api::DeleteSnapshotResponse>> {
        dbg!(request.into_inner());
        crate::todo!("delete_snapshot")
    }

    async fn list_snapshots(
        &self,
        request: Request<::data_pond_api::ListSnapshotsRequest>,
    ) -> Result<Response<::data_pond_api::ListSnapshotsResponse>> {
        dbg!(request.into_inner());
        crate::todo!("list_snapshots")
    }

    async fn get_snapshot(
        &self,
        request: Request<::data_pond_api::GetSnapshotRequest>,
    ) -> Result<Response<::data_pond_api::GetSnapshotResponse>> {
        dbg!(request.into_inner());
        crate::todo!("get_snapshot")
    }

    async fn controller_expand_volume(
        &self,
        request: Request<::data_pond_api::ControllerExpandVolumeRequest>,
    ) -> Result<Response<::data_pond_api::ControllerExpandVolumeResponse>> {
        dbg!(request.into_inner());
        crate::todo!("controller_expand_volume")
    }

    async fn controller_get_volume(
        &self,
        request: Request<::data_pond_api::ControllerGetVolumeRequest>,
    ) -> Result<Response<::data_pond_api::ControllerGetVolumeResponse>> {
        dbg!(request.into_inner());
        crate::todo!("controller_get_volume")
    }

    async fn controller_modify_volume(
        &self,
        request: Request<::data_pond_api::ControllerModifyVolumeRequest>,
    ) -> Result<Response<::data_pond_api::ControllerModifyVolumeResponse>> {
        dbg!(request.into_inner());
        crate::todo!("controller_modify_volume")
    }
}
