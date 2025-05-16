use std::collections::HashMap;

use async_trait::async_trait;
use data_pond_api::csi::{self, node_server::Node};
use tonic::{Request, Response, Result};

#[async_trait]
impl Node for super::Server {
    async fn node_stage_volume(
        &self,
        request: Request<csi::NodeStageVolumeRequest>,
    ) -> Result<Response<csi::NodeStageVolumeResponse>> {
        dbg!(request.into_inner());
        crate::todo!("node_stage_volume")
    }

    async fn node_unstage_volume(
        &self,
        request: Request<csi::NodeUnstageVolumeRequest>,
    ) -> Result<Response<csi::NodeUnstageVolumeResponse>> {
        dbg!(request.into_inner());
        crate::todo!("node_unstage_volume")
    }

    async fn node_publish_volume(
        &self,
        request: Request<csi::NodePublishVolumeRequest>,
    ) -> Result<Response<csi::NodePublishVolumeResponse>> {
        dbg!(request.into_inner());
        crate::todo!("node_publish_volume")
    }

    async fn node_unpublish_volume(
        &self,
        request: Request<csi::NodeUnpublishVolumeRequest>,
    ) -> Result<Response<csi::NodeUnpublishVolumeResponse>> {
        dbg!(request.into_inner());
        crate::todo!("node_unpublish_volume")
    }

    async fn node_get_volume_stats(
        &self,
        request: Request<csi::NodeGetVolumeStatsRequest>,
    ) -> Result<Response<csi::NodeGetVolumeStatsResponse>> {
        dbg!(request.into_inner());
        crate::todo!("node_get_volume_stats")
    }

    async fn node_expand_volume(
        &self,
        request: Request<csi::NodeExpandVolumeRequest>,
    ) -> Result<Response<csi::NodeExpandVolumeResponse>> {
        dbg!(request.into_inner());
        crate::todo!("node_expand_volume")
    }

    async fn node_get_capabilities(
        &self,
        request: Request<csi::NodeGetCapabilitiesRequest>,
    ) -> Result<Response<csi::NodeGetCapabilitiesResponse>> {
        dbg!(request.into_inner());
        crate::todo!("node_get_capabilities")
    }

    async fn node_get_info(
        &self,
        request: Request<csi::NodeGetInfoRequest>,
    ) -> Result<Response<csi::NodeGetInfoResponse>> {
        let csi::NodeGetInfoRequest {} = request.into_inner();

        Ok(Response::new(csi::NodeGetInfoResponse {
            node_id: self.node_id.clone(),
            max_volumes_per_node: 0, // delegate to CO
            accessible_topology: Some(csi::Topology {
                segments: HashMap::default(),
            }),
        }))
    }
}
