mod nvme;

use anyhow::Result;

pub(crate) async fn discover(server: &super::Server) -> Result<super::Endpoint> {
    Ok(super::Endpoint {
        devices: self::nvme::discover(server).await?,
    })
}
