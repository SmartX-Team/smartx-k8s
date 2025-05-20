use std::{collections::HashMap, process::Stdio};

use async_trait::async_trait;
use data_pond_csi::pond::{self, pond_server::Pond};
use tokio::{io::AsyncWriteExt, process::Command};
use tonic::{Request, Response, Result, Status};

impl super::Server {
    async fn execute(
        &self,
        kind: &str,
        request: Request<pond::AllocateVolumeRequest>,
    ) -> Result<Response<pond::AllocateVolumeResponse>> {
        let request = request.into_inner();
        let pond::AllocateVolumeRequest {
            device_id,
            volume_id,
            options,
            ..
        } = &request;

        // Validate options
        let options = options
            .as_ref()
            .ok_or_else(|| Status::invalid_argument("Empty options"))?;
        let layer = options.layer();

        // Load a device
        let devices = self.state.devices.read().await;
        if !devices.contains_key(device_id) {
            return Err(Status::not_found(format!("No such device: {device_id}")));
        }

        // Execute a program
        let program = format!("./{layer}-{kind}.sh");
        let mut process = Command::new(program)
            .stdin(Stdio::piped())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?;

        // Serialize inputs
        let inputs = ::serde_json::to_vec(&request)
            .map_err(|_| Status::internal("Failed to serialize the request"))?;

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
        self.execute("allocate", request).await
    }

    #[inline]
    async fn deallocate_volume(
        &self,
        request: Request<pond::AllocateVolumeRequest>,
    ) -> Result<Response<pond::AllocateVolumeResponse>> {
        self.execute("deallocate", request).await
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
