use async_trait::async_trait;
use data_pond_api::pond::{self, pond_server::Pond};
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
        }))
    }
}
