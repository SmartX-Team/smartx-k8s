use std::{collections::HashMap, sync::atomic::Ordering};

use async_trait::async_trait;
use data_pond_api::identity_server::Identity;
use tonic::{Request, Response, Result};

#[async_trait]
impl Identity for super::Server {
    async fn get_plugin_info(
        &self,
        request: Request<::data_pond_api::GetPluginInfoRequest>,
    ) -> Result<Response<::data_pond_api::GetPluginInfoResponse>> {
        let ::data_pond_api::GetPluginInfoRequest {} = request.into_inner();

        Ok(Response::new(::data_pond_api::GetPluginInfoResponse {
            name: "data-pond".into(),
            vendor_version: env!("CARGO_PKG_VERSION").into(),
            manifest: HashMap::default(),
        }))
    }

    async fn get_plugin_capabilities(
        &self,
        request: Request<::data_pond_api::GetPluginCapabilitiesRequest>,
    ) -> Result<Response<::data_pond_api::GetPluginCapabilitiesResponse>> {
        let ::data_pond_api::GetPluginCapabilitiesRequest {} = request.into_inner();

        Ok(Response::new(
            ::data_pond_api::GetPluginCapabilitiesResponse {
                capabilities: vec![
                    ::data_pond_api::PluginCapability {
                        r#type: Some(::data_pond_api::plugin_capability::Type::Service(
                            ::data_pond_api::plugin_capability::Service {
                                r#type:
                                    ::data_pond_api::plugin_capability::service::Type::ControllerService
                                        as _,
                            },
                        )),
                    },
                    ::data_pond_api::PluginCapability {
                        r#type: Some(::data_pond_api::plugin_capability::Type::Service(
                            ::data_pond_api::plugin_capability::Service {
                                r#type:
                                    ::data_pond_api::plugin_capability::service::Type::VolumeAccessibilityConstraints
                                        as _,
                            },
                        )),
                    },
                    ::data_pond_api::PluginCapability {
                        r#type: Some(::data_pond_api::plugin_capability::Type::VolumeExpansion(
                            ::data_pond_api::plugin_capability::VolumeExpansion {
                                r#type:
                                    ::data_pond_api::plugin_capability::volume_expansion::Type::Online
                                        as _,
                            },
                        )),
                    },
                ],
            },
        ))
    }

    async fn probe(
        &self,
        request: Request<::data_pond_api::ProbeRequest>,
    ) -> Result<Response<::data_pond_api::ProbeResponse>> {
        let ::data_pond_api::ProbeRequest {} = request.into_inner();

        Ok(Response::new(::data_pond_api::ProbeResponse {
            ready: Some(self.state.ready.load(Ordering::SeqCst)),
        }))
    }
}
