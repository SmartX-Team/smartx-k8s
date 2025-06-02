use std::collections::HashMap;

use async_trait::async_trait;
use data_pond_api::{VolumeAllocateContext, VolumeBindingContext};
use data_pond_csi::pond::{self, pond_server::Pond};
use tonic::{Request, Response, Result, Status};
#[cfg(feature = "tracing")]
use tracing::debug;

use crate::volume::{PondVolumeAllocate, VolumeParametersSource};

impl super::Server {
    pub(crate) async fn handle_pond_allocate(
        &self,
        kind: &str,
        request: pond::AllocateVolumeRequest,
    ) -> Result<Response<pond::AllocateVolumeResponse>> {
        // ****************************************
        // Step 0: Validate inputs
        // ****************************************

        #[cfg(feature = "tracing")]
        debug!("request = {request:#?}");

        let pond::AllocateVolumeRequest {
            binding,
            device_id,
            options,
            secrets,
        } = request;

        // ****************************************
        // Step 1: Validate volume options
        // ****************************************

        let options = options.ok_or_else(|| Status::invalid_argument("Empty options"))?;
        let binding = binding.ok_or_else(|| Status::invalid_argument("Empty binding"))?;
        let volume_id = binding.volume_id.clone();

        // ****************************************
        // Step 2: Validate volume parameters
        // ****************************************

        let secrets = secrets.parse()?;

        // ****************************************
        // Step 3: [C] Validate device
        // ****************************************

        // Load a device
        let mut state = self.state.write().await;
        let device = state
            .devices
            .get(&device_id)
            .ok_or_else(|| Status::not_found(format!("No such device: {device_id}")))?;

        // ****************************************
        // Step 4: [C, E] Execute
        // ****************************************

        // Build context
        let binding = VolumeBindingContext {
            addr: self.pod_ip.to_string(),
            device: device.clone(),
            layer: device.layer(),
            metadata: binding.clone(),
            source: device.source(),
        };
        let context = VolumeAllocateContext {
            binding: &binding,
            options: &options,
            secrets: &secrets,
        };

        // Execute a program
        context.execute(kind).await?;

        // Store the volume
        {
            state.bindings.insert(volume_id, binding.metadata);
            drop(state);
        }
        Ok(Response::new(pond::AllocateVolumeResponse {}))
    }
}

#[async_trait]
impl Pond for super::Server {
    #[inline]
    async fn allocate_volume(
        &self,
        request: Request<pond::AllocateVolumeRequest>,
    ) -> Result<Response<pond::AllocateVolumeResponse>> {
        self.handle_pond_allocate("allocate", request.into_inner())
            .await
    }

    #[inline]
    async fn deallocate_volume(
        &self,
        request: Request<pond::AllocateVolumeRequest>,
    ) -> Result<Response<pond::AllocateVolumeResponse>> {
        self.handle_pond_allocate("deallocate", request.into_inner())
            .await
    }

    async fn list_devices(
        &self,
        request: Request<pond::ListDevicesRequest>,
    ) -> Result<Response<pond::ListDevicesResponse>> {
        let pond::ListDevicesRequest {} = request.into_inner();

        let state = self.state.read().await;
        Ok(Response::new(pond::ListDevicesResponse {
            id: self.node_id.clone(),
            bindings: state.bindings.values().cloned().collect(),
            devices: state.devices.values().cloned().collect(),
            topology: Some(pond::DeviceTopology {
                required: HashMap::default(),
                provides: self.node_topology(),
            }),
        }))
    }
}
