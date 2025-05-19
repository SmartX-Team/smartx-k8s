use std::{net::IpAddr, ops};

use async_trait::async_trait;
use data_pond_api::csi::{self, controller_server::Controller};
use hickory_resolver::{
    ResolveError, Resolver,
    name_server::{GenericConnector, TokioConnectionProvider},
    proto::runtime::TokioRuntimeProvider,
};
use tonic::{Request, Response, Result, Status};

pub(crate) struct Server {
    inner: super::Server,
    resolver: Resolver<GenericConnector<TokioRuntimeProvider>>,
}

impl ops::Deref for Server {
    type Target = super::Server;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Server {
    pub(crate) async fn try_new(inner: super::Server) -> Result<Self, ResolveError> {
        // Construct a new Resolver with default configuration options
        let provider = TokioConnectionProvider::default();
        let resolver = Resolver::builder(provider)?.build();
        Ok(Self { inner, resolver })
    }

    async fn discover(&self) -> Result<Vec<IpAddr>> {
        match self
            .resolver
            .ipv4_lookup("plugin.hoya.svc.ops.openark")
            .await
        {
            Ok(lookup) => Ok(lookup.iter().map(|record| IpAddr::V4(record.0)).collect()),
            Err(error)
                if error
                    .proto()
                    .is_some_and(|proto| proto.kind().is_no_records_found()) =>
            {
                Ok(Default::default())
            }
            Err(error) => Err(Status::internal(error.to_string())),
        }
    }
}

#[async_trait]
impl Controller for Server {
    async fn create_volume(
        &self,
        request: Request<csi::CreateVolumeRequest>,
    ) -> Result<Response<csi::CreateVolumeResponse>> {
        let request = request.into_inner();
        dbg!(request);

        // FIXME: To be implemented!
        // TODO: endpoints 로부터 주기적으로 데이터 가져오기
        // TODO: 1. DNS IP 조회
        // TODO: 2. TTL 반영해 리프레싱
        // TODO: 3. 덤프 띄워 반환
        dbg!(self.discover().await?);

        // Ok(Response::new(csi::CreateVolumeResponse {
        //     volume: Some(),
        // }))
        Err(Status::resource_exhausted("Cannot create volumes"))
    }

    async fn delete_volume(
        &self,
        request: Request<csi::DeleteVolumeRequest>,
    ) -> Result<Response<csi::DeleteVolumeResponse>> {
        dbg!(request.into_inner());
        crate::todo!("delete_volume")
    }

    async fn controller_publish_volume(
        &self,
        request: Request<csi::ControllerPublishVolumeRequest>,
    ) -> Result<Response<csi::ControllerPublishVolumeResponse>> {
        dbg!(request.into_inner());
        crate::todo!("controller_publish_volume")
    }

    async fn controller_unpublish_volume(
        &self,
        request: Request<csi::ControllerUnpublishVolumeRequest>,
    ) -> Result<Response<csi::ControllerUnpublishVolumeResponse>> {
        dbg!(request.into_inner());
        crate::todo!("controller_unpublish_volume")
    }

    async fn validate_volume_capabilities(
        &self,
        request: Request<csi::ValidateVolumeCapabilitiesRequest>,
    ) -> Result<Response<csi::ValidateVolumeCapabilitiesResponse>> {
        dbg!(request.into_inner());
        crate::todo!("validate_volume_capabilities")
    }

    async fn list_volumes(
        &self,
        request: Request<csi::ListVolumesRequest>,
    ) -> Result<Response<csi::ListVolumesResponse>> {
        let csi::ListVolumesRequest {
            max_entries,
            starting_token,
        } = request.into_inner();

        // Collect entries
        let max_entries = max_entries.max(0).min(i32::MAX - 1) as usize;
        let mut entries: Vec<_> = self
            .state
            .volumes
            .read()
            .await
            .range(starting_token..)
            .take(max_entries + 1)
            .map(|(_, value)| csi::list_volumes_response::Entry {
                volume: Some(value.data.clone()),
                status: Some(csi::list_volumes_response::VolumeStatus {
                    published_node_ids: value.published_node_ids.iter().cloned().collect(),
                    volume_condition: Some(value.condition.clone()),
                }),
            })
            .collect();

        // Pick up next token
        let next_token = if entries.len() == max_entries {
            entries
                .pop()
                .and_then(|entry| entry.volume)
                .map(|volume| volume.volume_id)
        } else {
            None
        };

        Ok(Response::new(csi::ListVolumesResponse {
            entries,
            next_token: next_token.unwrap_or_default(),
        }))
    }

    async fn get_capacity(
        &self,
        request: Request<csi::GetCapacityRequest>,
    ) -> Result<Response<csi::GetCapacityResponse>> {
        dbg!(request.into_inner());
        crate::todo!("get_capacity")
    }

    async fn controller_get_capabilities(
        &self,
        request: Request<csi::ControllerGetCapabilitiesRequest>,
    ) -> Result<Response<csi::ControllerGetCapabilitiesResponse>> {
        let csi::ControllerGetCapabilitiesRequest {} = request.into_inner();

        Ok(Response::new(
            csi::ControllerGetCapabilitiesResponse {
                capabilities: vec![
                    csi::ControllerServiceCapability {
                        r#type: Some(csi::controller_service_capability::Type::Rpc(
                            csi::controller_service_capability::Rpc {
                                r#type: csi::controller_service_capability::rpc::Type::CreateDeleteVolume as _,
                            },
                        )),
                    },
                    csi::ControllerServiceCapability {
                        r#type: Some(csi::controller_service_capability::Type::Rpc(
                            csi::controller_service_capability::Rpc {
                                r#type: csi::controller_service_capability::rpc::Type::GetCapacity as _,
                            },
                        )),
                    },
                    csi::ControllerServiceCapability {
                        r#type: Some(csi::controller_service_capability::Type::Rpc(
                            csi::controller_service_capability::Rpc {
                                r#type: csi::controller_service_capability::rpc::Type::GetVolume as _,
                            },
                        )),
                    },
                    csi::ControllerServiceCapability {
                        r#type: Some(csi::controller_service_capability::Type::Rpc(
                            csi::controller_service_capability::Rpc {
                                r#type: csi::controller_service_capability::rpc::Type::ExpandVolume as _,
                            },
                        )),
                    },
                    csi::ControllerServiceCapability {
                        r#type: Some(csi::controller_service_capability::Type::Rpc(
                            csi::controller_service_capability::Rpc {
                                r#type: csi::controller_service_capability::rpc::Type::ListVolumes as _,
                            },
                        )),
                    },
                    csi::ControllerServiceCapability {
                        r#type: Some(csi::controller_service_capability::Type::Rpc(
                            csi::controller_service_capability::Rpc {
                                r#type: csi::controller_service_capability::rpc::Type::ListVolumesPublishedNodes as _,
                            },
                        )),
                    },
                    csi::ControllerServiceCapability {
                        r#type: Some(csi::controller_service_capability::Type::Rpc(
                            csi::controller_service_capability::Rpc {
                                r#type: csi::controller_service_capability::rpc::Type::ModifyVolume as _,
                            },
                        )),
                    },
                    csi::ControllerServiceCapability {
                        r#type: Some(csi::controller_service_capability::Type::Rpc(
                            csi::controller_service_capability::Rpc {
                                r#type: csi::controller_service_capability::rpc::Type::PublishUnpublishVolume as _,
                            },
                        )),
                    },
                    csi::ControllerServiceCapability {
                        r#type: Some(csi::controller_service_capability::Type::Rpc(
                            csi::controller_service_capability::Rpc {
                                r#type: csi::controller_service_capability::rpc::Type::SingleNodeMultiWriter as _,
                            },
                        )),
                    },
                    csi::ControllerServiceCapability {
                        r#type: Some(csi::controller_service_capability::Type::Rpc(
                            csi::controller_service_capability::Rpc {
                                r#type: csi::controller_service_capability::rpc::Type::VolumeCondition as _,
                            },
                        )),
                    },
                ],
            },
        ))
    }

    async fn create_snapshot(
        &self,
        request: Request<csi::CreateSnapshotRequest>,
    ) -> Result<Response<csi::CreateSnapshotResponse>> {
        dbg!(request.into_inner());
        crate::todo!("create_snapshot")
    }

    async fn delete_snapshot(
        &self,
        request: Request<csi::DeleteSnapshotRequest>,
    ) -> Result<Response<csi::DeleteSnapshotResponse>> {
        dbg!(request.into_inner());
        crate::todo!("delete_snapshot")
    }

    async fn list_snapshots(
        &self,
        request: Request<csi::ListSnapshotsRequest>,
    ) -> Result<Response<csi::ListSnapshotsResponse>> {
        dbg!(request.into_inner());
        crate::todo!("list_snapshots")
    }

    async fn get_snapshot(
        &self,
        request: Request<csi::GetSnapshotRequest>,
    ) -> Result<Response<csi::GetSnapshotResponse>> {
        dbg!(request.into_inner());
        crate::todo!("get_snapshot")
    }

    async fn controller_expand_volume(
        &self,
        request: Request<csi::ControllerExpandVolumeRequest>,
    ) -> Result<Response<csi::ControllerExpandVolumeResponse>> {
        dbg!(request.into_inner());
        crate::todo!("controller_expand_volume")
    }

    async fn controller_get_volume(
        &self,
        request: Request<csi::ControllerGetVolumeRequest>,
    ) -> Result<Response<csi::ControllerGetVolumeResponse>> {
        dbg!(request.into_inner());
        crate::todo!("controller_get_volume")
    }

    async fn controller_modify_volume(
        &self,
        request: Request<csi::ControllerModifyVolumeRequest>,
    ) -> Result<Response<csi::ControllerModifyVolumeResponse>> {
        dbg!(request.into_inner());
        crate::todo!("controller_modify_volume")
    }
}
