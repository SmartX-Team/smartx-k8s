use std::collections::HashMap;

use async_trait::async_trait;
use data_pond_csi::pond::{self, pond_server::Pond};
use tonic::{Request, Response, Result};

#[async_trait]
impl Pond for super::Server {
    async fn list_devices(
        &self,
        request: Request<pond::ListDevicesRequest>,
    ) -> Result<Response<pond::ListDevicesResponse>> {
        let pond::ListDevicesRequest {} = request.into_inner();

        Ok(Response::new(pond::ListDevicesResponse {
            devices: self.state.devices.read().await.clone(),
            topology: Some(pond::DeviceTopology {
                required: HashMap::default(),
                provides: {
                    let mut map = HashMap::default();
                    map.insert("kubernetes.io/hostname".into(), self.node_id.clone());
                    map
                },
            }),
        }))
    }
}
