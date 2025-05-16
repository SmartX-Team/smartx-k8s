use async_trait::async_trait;
use data_pond_api::pond::{self, pond_server::Pond};
use tonic::{Request, Response, Result, Status};

#[async_trait]
impl Pond for super::Server {
    async fn list_devices(
        &self,
        request: Request<pond::ListDevicesRequest>,
    ) -> Result<Response<pond::ListDevicesResponse>> {
        let request = request.into_inner();
        dbg!(request);

        // FIXME: To be implemented!
        Err(Status::resource_exhausted("Cannot list devices"))
    }
}
