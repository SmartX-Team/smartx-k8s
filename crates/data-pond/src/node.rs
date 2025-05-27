use std::{collections::HashMap, process::Stdio};

use async_trait::async_trait;
use data_pond_api::{
    VolumeAllocateContext, VolumePublishContext, VolumePublishControllerContext,
    VolumeUnpublishContext,
};
use data_pond_csi::csi::{self, node_server::Node};
use tokio::{io::AsyncWriteExt, process::Command};
use tonic::{Request, Response, Result, Status};
#[cfg(feature = "tracing")]
use tracing::{debug, warn};

use crate::volume::{PondVolumeAllocate, VolumeOptionsExt, VolumeParametersSource};

#[derive(Debug)]
struct NodePublishVolumeRequest {
    publish_context: HashMap<String, String>,
    read_only: bool,
    secrets: HashMap<String, String>,
    staging_target_path: String,
    target_path: Option<String>,
    volume_capability: Option<csi::VolumeCapability>,
    volume_id: String,
}

impl From<csi::NodeStageVolumeRequest> for NodePublishVolumeRequest {
    fn from(value: csi::NodeStageVolumeRequest) -> Self {
        let csi::NodeStageVolumeRequest {
            volume_id,
            publish_context,
            staging_target_path,
            volume_capability,
            secrets,
            volume_context: _,
        } = value;

        Self {
            publish_context,
            read_only: false,
            secrets,
            staging_target_path,
            target_path: None,
            volume_capability,
            volume_id,
        }
    }
}

impl From<csi::NodePublishVolumeRequest> for NodePublishVolumeRequest {
    fn from(value: csi::NodePublishVolumeRequest) -> Self {
        let csi::NodePublishVolumeRequest {
            volume_id,
            publish_context,
            staging_target_path,
            target_path,
            volume_capability,
            readonly,
            secrets,
            volume_context: _,
        } = value;

        Self {
            publish_context,
            read_only: readonly,
            secrets,
            staging_target_path,
            target_path: Some(target_path),
            volume_capability,
            volume_id,
        }
    }
}

#[derive(Debug)]
struct NodeUnpublishVolumeRequest {
    target_path: String,
    volume_id: String,
}

impl From<csi::NodeUnstageVolumeRequest> for NodeUnpublishVolumeRequest {
    fn from(value: csi::NodeUnstageVolumeRequest) -> Self {
        let csi::NodeUnstageVolumeRequest {
            volume_id,
            staging_target_path,
        } = value;

        Self {
            target_path: staging_target_path,
            volume_id,
        }
    }
}

impl From<csi::NodeUnpublishVolumeRequest> for NodeUnpublishVolumeRequest {
    fn from(value: csi::NodeUnpublishVolumeRequest) -> Self {
        let csi::NodeUnpublishVolumeRequest {
            volume_id,
            target_path,
        } = value;

        Self {
            target_path,
            volume_id,
        }
    }
}

impl super::Server {
    async fn handle_node_publish<R>(&self, kind: &str, request: Request<R>) -> Result<()>
    where
        R: Into<NodePublishVolumeRequest>,
    {
        // ****************************************
        // Step 0: Validate inputs
        // ****************************************

        let request = request.into_inner().into();
        #[cfg(feature = "tracing")]
        debug!("request = {request:#?}");

        let NodePublishVolumeRequest {
            publish_context,
            read_only,
            secrets,
            staging_target_path,
            target_path,
            volume_capability,
            volume_id,
        } = request;

        // ****************************************
        // Step 1: Validate volume options
        // ****************************************

        let mut options = self.default.volume_options();
        options.apply_volume_capability(volume_capability.unwrap_or_default())?;

        // ****************************************
        // Step 2: Validate volume parameters
        // ****************************************

        // Validate parameters
        let controller: VolumePublishControllerContext = publish_context.parse()?;
        let secrets = secrets.parse()?;

        // ****************************************
        // Step 3: [E] Allocate volumes
        // ****************************************

        for binding in &controller.bindings {
            if binding.metadata.device_id != binding.device.id {
                return Err(Status::internal(format!(
                    "Invalid device ID: expected {expected:?}, but given {given:?}",
                    expected = binding.device.id,
                    given = binding.metadata.device_id,
                )));
            }
            if binding.metadata.volume_id != volume_id {
                return Err(Status::internal(format!(
                    "Invalid device volume ID ({device_id}): expected {expected:?}, but given {given:?}",
                    device_id = binding.metadata.device_id,
                    expected = volume_id,
                    given = binding.metadata.volume_id,
                )));
            }

            let context = VolumeAllocateContext {
                binding,
                options: &options,
                secrets: &secrets,
            };
            context.allocate().await?;
        }

        // ****************************************
        // Step 4: [E] Execute
        // ****************************************

        // Build context
        let context = VolumePublishContext {
            controller,
            options,
            read_only,
            secrets,
            staging_target_path,
            target_path,
            volume_id,
        };

        // Execute a program
        let layer = context.controller.layer;
        let program = format!("./{layer}-{kind}.sh");
        let mut process = Command::new(program)
            .stdin(Stdio::piped())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?;

        // Serialize inputs
        let inputs = ::serde_json::to_vec(&context)
            .map_err(|_| Status::internal("Failed to serialize the context"))?;

        // Feed inputs
        {
            let mut stdin = process.stdin.take().unwrap();
            stdin.write_all(&inputs).await?;
            stdin.flush().await?;
        }

        // Validate the process
        let status = process.wait().await?;
        if status.success() {
            Ok(())
        } else {
            Err(Status::internal(format!(
                "Failed to {kind} {volume_id}: {code}",
                code = status.code().unwrap_or(-1),
                volume_id = context.volume_id,
            )))
        }
    }

    async fn handle_node_unpublish<R>(&self, kind: &str, request: Request<R>) -> Result<()>
    where
        R: Into<NodeUnpublishVolumeRequest>,
    {
        // ****************************************
        // Step 0: Validate inputs
        // ****************************************

        let request = request.into_inner().into();
        #[cfg(feature = "tracing")]
        debug!("request = {request:#?}");

        let NodeUnpublishVolumeRequest {
            target_path,
            volume_id,
        } = request;

        // ****************************************
        // Step 1: [E] Execute
        // ****************************************

        // Build context
        let context = VolumeUnpublishContext {
            target_path,
            volume_id,
        };

        // Execute a program
        let program = format!("./generic-{kind}.sh");
        let mut process = Command::new(program)
            .stdin(Stdio::piped())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?;

        // Serialize inputs
        let inputs = ::serde_json::to_vec(&context)
            .map_err(|_| Status::internal("Failed to serialize the context"))?;

        // Feed inputs
        {
            let mut stdin = process.stdin.take().unwrap();
            stdin.write_all(&inputs).await?;
            stdin.flush().await?;
        }

        // Validate the process
        let status = process.wait().await?;
        if status.success() {
            Ok(())
        } else {
            Err(Status::internal(format!(
                "Failed to {kind} {volume_id}: {code}",
                code = status.code().unwrap_or(-1),
                volume_id = context.volume_id,
            )))
        }
    }
}

#[async_trait]
impl Node for super::Server {
    async fn node_stage_volume(
        &self,
        request: Request<csi::NodeStageVolumeRequest>,
    ) -> Result<Response<csi::NodeStageVolumeResponse>> {
        self.handle_node_publish("stage", request).await?;
        Ok(Response::new(csi::NodeStageVolumeResponse {}))
    }

    async fn node_unstage_volume(
        &self,
        request: Request<csi::NodeUnstageVolumeRequest>,
    ) -> Result<Response<csi::NodeUnstageVolumeResponse>> {
        self.handle_node_unpublish("unstage", request).await?;
        Ok(Response::new(csi::NodeUnstageVolumeResponse {}))
    }

    async fn node_publish_volume(
        &self,
        request: Request<csi::NodePublishVolumeRequest>,
    ) -> Result<Response<csi::NodePublishVolumeResponse>> {
        self.handle_node_publish("publish", request).await?;
        Ok(Response::new(csi::NodePublishVolumeResponse {}))
    }

    async fn node_unpublish_volume(
        &self,
        request: Request<csi::NodeUnpublishVolumeRequest>,
    ) -> Result<Response<csi::NodeUnpublishVolumeResponse>> {
        self.handle_node_unpublish("unpublish", request).await?;
        Ok(Response::new(csi::NodeUnpublishVolumeResponse {}))
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
