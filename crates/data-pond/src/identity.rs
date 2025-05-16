use std::collections::HashMap;

use async_trait::async_trait;
use data_pond_api::csi::{self, identity_server::Identity};
use tonic::{Request, Response, Result};

#[async_trait]
impl Identity for super::Server {
    async fn get_plugin_info(
        &self,
        request: Request<csi::GetPluginInfoRequest>,
    ) -> Result<Response<csi::GetPluginInfoResponse>> {
        let csi::GetPluginInfoRequest {} = request.into_inner();

        Ok(Response::new(csi::GetPluginInfoResponse {
            name: self.driver_name.clone(),
            vendor_version: env!("CARGO_PKG_VERSION").into(),
            manifest: HashMap::default(),
        }))
    }

    async fn get_plugin_capabilities(
        &self,
        request: Request<csi::GetPluginCapabilitiesRequest>,
    ) -> Result<Response<csi::GetPluginCapabilitiesResponse>> {
        let csi::GetPluginCapabilitiesRequest {} = request.into_inner();

        Ok(Response::new(
            csi::GetPluginCapabilitiesResponse {
                capabilities: vec![
                    csi::PluginCapability {
                        r#type: Some(csi::plugin_capability::Type::Service(
                            csi::plugin_capability::Service {
                                r#type:
                                    csi::plugin_capability::service::Type::ControllerService as _,
                            },
                        )),
                    },
                    csi::PluginCapability {
                        r#type: Some(csi::plugin_capability::Type::Service(
                            csi::plugin_capability::Service {
                                r#type:
                                    csi::plugin_capability::service::Type::VolumeAccessibilityConstraints as _,
                            },
                        )),
                    },
                    csi::PluginCapability {
                        r#type: Some(csi::plugin_capability::Type::VolumeExpansion(
                            csi::plugin_capability::VolumeExpansion {
                                r#type:
                                    csi::plugin_capability::volume_expansion::Type::Online as _,
                            },
                        )),
                    },
                ],
            },
        ))
    }

    async fn probe(
        &self,
        request: Request<csi::ProbeRequest>,
    ) -> Result<Response<csi::ProbeResponse>> {
        let csi::ProbeRequest {} = request.into_inner();

        Ok(Response::new(csi::ProbeResponse { ready: Some(true) }))
    }
}
