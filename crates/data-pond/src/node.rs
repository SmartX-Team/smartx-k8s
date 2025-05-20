use async_trait::async_trait;
use data_pond_csi::csi::{self, node_server::Node};
use tonic::{Request, Response, Result};
#[cfg(feature = "tracing")]
use tracing::warn;

#[async_trait]
impl Node for super::Server {
    async fn node_stage_volume(
        &self,
        request: Request<csi::NodeStageVolumeRequest>,
    ) -> Result<Response<csi::NodeStageVolumeResponse>> {
        // let request = request.into_inner();
        // let csi::NodeStageVolumeRequest { volume_id, .. } = &request;

        // FIXME: To be implemented!
        #[cfg(feature = "tracing")]
        warn!("request = {:#?}", request.into_inner());
        crate::todo!("node_stage_volume")
    }

    async fn node_unstage_volume(
        &self,
        request: Request<csi::NodeUnstageVolumeRequest>,
    ) -> Result<Response<csi::NodeUnstageVolumeResponse>> {
        // FIXME: To be implemented!
        #[cfg(feature = "tracing")]
        warn!("request = {:#?}", request.into_inner());
        crate::todo!("node_unstage_volume")
    }

    async fn node_publish_volume(
        &self,
        request: Request<csi::NodePublishVolumeRequest>,
    ) -> Result<Response<csi::NodePublishVolumeResponse>> {
        // FIXME: To be implemented!
        #[cfg(feature = "tracing")]
        warn!("request = {:#?}", request.into_inner());
        crate::todo!("node_publish_volume")
    }

    async fn node_unpublish_volume(
        &self,
        request: Request<csi::NodeUnpublishVolumeRequest>,
    ) -> Result<Response<csi::NodeUnpublishVolumeResponse>> {
        // FIXME: To be implemented!
        #[cfg(feature = "tracing")]
        warn!("request = {:#?}", request.into_inner());
        crate::todo!("node_unpublish_volume")
    }

    async fn node_get_volume_stats(
        &self,
        request: Request<csi::NodeGetVolumeStatsRequest>,
    ) -> Result<Response<csi::NodeGetVolumeStatsResponse>> {
        // FIXME: To be implemented!
        #[cfg(feature = "tracing")]
        warn!("request = {:#?}", request.into_inner());
        crate::todo!("node_get_volume_stats")
    }

    async fn node_expand_volume(
        &self,
        request: Request<csi::NodeExpandVolumeRequest>,
    ) -> Result<Response<csi::NodeExpandVolumeResponse>> {
        // FIXME: To be implemented!
        #[cfg(feature = "tracing")]
        warn!("request = {:#?}", request.into_inner());
        crate::todo!("node_expand_volume")
    }

    async fn node_get_capabilities(
        &self,
        request: Request<csi::NodeGetCapabilitiesRequest>,
    ) -> Result<Response<csi::NodeGetCapabilitiesResponse>> {
        let csi::NodeGetCapabilitiesRequest {} = request.into_inner();

        Ok(Response::new(csi::NodeGetCapabilitiesResponse {
            capabilities: vec![
                csi::NodeServiceCapability {
                    r#type: Some(csi::node_service_capability::Type::Rpc(
                        csi::node_service_capability::Rpc {
                            r#type: csi::node_service_capability::rpc::Type::StageUnstageVolume
                                as _,
                        },
                    )),
                },
                csi::NodeServiceCapability {
                    r#type: Some(csi::node_service_capability::Type::Rpc(
                        csi::node_service_capability::Rpc {
                            r#type: csi::node_service_capability::rpc::Type::GetVolumeStats as _,
                        },
                    )),
                },
                csi::NodeServiceCapability {
                    r#type: Some(csi::node_service_capability::Type::Rpc(
                        csi::node_service_capability::Rpc {
                            r#type: csi::node_service_capability::rpc::Type::ExpandVolume as _,
                        },
                    )),
                },
                csi::NodeServiceCapability {
                    r#type: Some(csi::node_service_capability::Type::Rpc(
                        csi::node_service_capability::Rpc {
                            r#type: csi::node_service_capability::rpc::Type::VolumeCondition as _,
                        },
                    )),
                },
                csi::NodeServiceCapability {
                    r#type: Some(csi::node_service_capability::Type::Rpc(
                        csi::node_service_capability::Rpc {
                            r#type: csi::node_service_capability::rpc::Type::SingleNodeMultiWriter
                                as _,
                        },
                    )),
                },
            ],
        }))
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
                segments: self.node_topology(),
            }),
        }))
    }
}
