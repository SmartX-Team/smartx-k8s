use async_trait::async_trait;
use data_pond_api::csi::{self, controller_server::Controller};
use tonic::{Request, Response, Result, Status};

#[async_trait]
impl Controller for super::Server {
    async fn create_volume(
        &self,
        request: Request<csi::CreateVolumeRequest>,
    ) -> Result<Response<csi::CreateVolumeResponse>> {
        let request = request.into_inner();
        dbg!(request);

        // Ok(Response::new(csi::CreateVolumeResponse {
        //     // FIXME: To be implemented!
        //     volume: None,
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
        let max_entries = max_entries.max(0).min(i32::MAX - 1) as _;
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
