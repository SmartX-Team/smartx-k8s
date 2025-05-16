use std::collections::HashMap;

use async_trait::async_trait;
use data_pond_api::node_server::Node;
use tonic::{Request, Response, Result};

#[async_trait]
impl Node for super::Server {
    async fn node_stage_volume(
        &self,
        request: Request<::data_pond_api::NodeStageVolumeRequest>,
    ) -> Result<Response<::data_pond_api::NodeStageVolumeResponse>> {
        dbg!(request.into_inner());
        crate::todo!("node_stage_volume")
    }

    async fn node_unstage_volume(
        &self,
        request: Request<::data_pond_api::NodeUnstageVolumeRequest>,
    ) -> Result<Response<::data_pond_api::NodeUnstageVolumeResponse>> {
        dbg!(request.into_inner());
        crate::todo!("node_unstage_volume")
    }

    async fn node_publish_volume(
        &self,
        request: Request<::data_pond_api::NodePublishVolumeRequest>,
    ) -> Result<Response<::data_pond_api::NodePublishVolumeResponse>> {
        dbg!(request.into_inner());
        crate::todo!("node_publish_volume")
    }

    async fn node_unpublish_volume(
        &self,
        request: Request<::data_pond_api::NodeUnpublishVolumeRequest>,
    ) -> Result<Response<::data_pond_api::NodeUnpublishVolumeResponse>> {
        dbg!(request.into_inner());
        crate::todo!("node_unpublish_volume")
    }

    async fn node_get_volume_stats(
        &self,
        request: Request<::data_pond_api::NodeGetVolumeStatsRequest>,
    ) -> Result<Response<::data_pond_api::NodeGetVolumeStatsResponse>> {
        dbg!(request.into_inner());
        crate::todo!("node_get_volume_stats")
    }

    async fn node_expand_volume(
        &self,
        request: Request<::data_pond_api::NodeExpandVolumeRequest>,
    ) -> Result<Response<::data_pond_api::NodeExpandVolumeResponse>> {
        dbg!(request.into_inner());
        crate::todo!("node_expand_volume")
    }

    async fn node_get_capabilities(
        &self,
        request: Request<::data_pond_api::NodeGetCapabilitiesRequest>,
    ) -> Result<Response<::data_pond_api::NodeGetCapabilitiesResponse>> {
        dbg!(request.into_inner());
        crate::todo!("node_get_capabilities")
    }

    async fn node_get_info(
        &self,
        request: Request<::data_pond_api::NodeGetInfoRequest>,
    ) -> Result<Response<::data_pond_api::NodeGetInfoResponse>> {
        let ::data_pond_api::NodeGetInfoRequest {} = request.into_inner();

        Ok(Response::new(::data_pond_api::NodeGetInfoResponse {
            node_id: todo!(),
            max_volumes_per_node: 0,
            accessible_topology: Some(::data_pond_api::Topology {
                segments: HashMap::default(),
            }),
        }))
    }
}
