use std::{collections::HashMap, process::Stdio};

use async_trait::async_trait;
use data_pond_api::{VolumeAllocateContext, VolumeContext, VolumeParameters};
use data_pond_csi::pond::{self, pond_server::Pond};
use tokio::{io::AsyncWriteExt, process::Command};
use tonic::{Request, Response, Result, Status};
#[cfg(feature = "tracing")]
use tracing::debug;

use crate::volume::VolumeParametersSource;

impl super::Server {
    async fn handle_pond_allocate(
        &self,
        kind: &str,
        request: Request<pond::AllocateVolumeRequest>,
    ) -> Result<Response<pond::AllocateVolumeResponse>> {
        // ****************************************
        // Step 0: Validate inputs
        // ****************************************

        let request = request.into_inner();
        #[cfg(feature = "tracing")]
        debug!("request = {request:#?}");

        let pond::AllocateVolumeRequest {
            device_id,
            volume_id,
            capacity,
            options,
            attributes,
            secrets,
        } = request;

        // ****************************************
        // Step 1: Validate volume options
        // ****************************************

        let options = options.ok_or_else(|| Status::invalid_argument("Empty options"))?;

        // ****************************************
        // Step 2: Validate volume parameters
        // ****************************************

        let attributes = vec![attributes].parse()?;
        let secrets = secrets.parse()?;

        // ****************************************
        // Step 3: Validate device
        // ****************************************

        // Load a device
        let devices = self.state.devices.read().await;
        if !devices.contains_key(&device_id) {
            return Err(Status::not_found(format!("No such device: {device_id}")));
        }

        // ****************************************
        // Step 4: [E] Execute
        // ****************************************

        // Build context
        let context = VolumeAllocateContext {
            capacity,
            device_id,
            volume: VolumeContext {
                id: volume_id,
                options,
                parameters: VolumeParameters {
                    attributes,
                    secrets,
                },
            },
        };

        // Execute a program
        let layer = context.volume.parameters.attributes.layer;
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
            drop(devices);
            Ok(Response::new(pond::AllocateVolumeResponse {}))
        } else {
            Err(Status::internal(format!(
                "Failed to {kind} {volume_id} into {device_id}: {code}",
                code = status.code().unwrap_or(-1),
                device_id = context.device_id,
                volume_id = context.volume.id,
            )))
        }
    }
}

#[async_trait]
impl Pond for super::Server {
    #[inline]
    async fn allocate_volume(
        &self,
        request: Request<pond::AllocateVolumeRequest>,
    ) -> Result<Response<pond::AllocateVolumeResponse>> {
        self.handle_pond_allocate("allocate", request).await
    }

    #[inline]
    async fn deallocate_volume(
        &self,
        request: Request<pond::AllocateVolumeRequest>,
    ) -> Result<Response<pond::AllocateVolumeResponse>> {
        self.handle_pond_allocate("deallocate", request).await
    }

    async fn list_devices(
        &self,
        request: Request<pond::ListDevicesRequest>,
    ) -> Result<Response<pond::ListDevicesResponse>> {
        let pond::ListDevicesRequest {} = request.into_inner();

        Ok(Response::new(pond::ListDevicesResponse {
            id: self.node_id.clone(),
            devices: self.state.devices.read().await.values().cloned().collect(),
            topology: Some(pond::DeviceTopology {
                required: HashMap::default(),
                provides: self.node_topology(),
            }),
        }))
    }
}
