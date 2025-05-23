use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use anyhow::Error;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use data_pond_api::{VolumeAttributes, VolumePublishControllerContext, VolumeSecrets};
use data_pond_csi::{
    csi::{self, controller_server::Controller},
    pond::{self, pond_client::PondClient},
};
use futures::{TryStreamExt, stream::FuturesUnordered};
use hickory_resolver::{
    Resolver,
    name_server::{GenericConnector, TokioConnectionProvider},
    proto::runtime::TokioRuntimeProvider,
};
use tokio::sync::{Mutex, RwLock, RwLockWriteGuard};
use tonic::{
    Request, Response, Result, Status,
    transport::{Channel, Uri},
};
#[cfg(feature = "tracing")]
use tracing::{debug, warn};

use crate::volume::{
    PersistentVolume, Pond, Volume, VolumeGroup, VolumeOptionsExt, VolumeParametersExport,
    VolumeParametersSource,
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct DeviceKey {
    pond_id: String,
    device_id: String,
}

#[derive(Clone, Debug, Default)]
struct Ponds {
    created_at: DateTime<Utc>,
    data: BTreeMap<String, Arc<Pond>>,
}

#[derive(Debug, Default)]
struct State {
    allocated: HashMap<DeviceKey, i64>,
    ponds: Ponds,
    volumes: BTreeMap<String, PersistentVolume>,
}

pub(crate) struct Server {
    default: super::DefaultSettings,
    pond_host: String,
    pond_port: u16,
    pond_protocol: String,
    pond_ttl: Duration,
    resolver: Resolver<GenericConnector<TokioRuntimeProvider>>,
    state: RwLock<State>,
}

impl Server {
    pub(crate) async fn try_new(default: super::DefaultSettings) -> Result<Self, Error> {
        // Construct a new Resolver with default configuration options
        let provider = TokioConnectionProvider::default();
        let resolver = Resolver::builder(provider)?.build();

        // Initialize ponds
        let server = Self {
            default,
            pond_host: "plugin.hoya.svc.ops.openark".into(),
            pond_port: 9090,
            pond_protocol: "http".into(),
            pond_ttl: Duration::seconds(30),
            resolver,
            state: Default::default(),
        };
        {
            let state = server.state.write().await;
            let _ = server.fetch_ponds(state).await?;
        };
        Ok(server)
    }

    async fn discover(&self) -> Result<Vec<Uri>> {
        match self.resolver.ipv4_lookup(&self.pond_host).await {
            Ok(lookup) => {
                let port = self.pond_port;
                let protocol = &self.pond_protocol;
                Ok(lookup
                    .iter()
                    .filter_map(|record| format!("{protocol}://{record}:{port}").parse().ok())
                    .collect())
            }
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

    async fn fetch_ponds<'a>(
        &self,
        mut state: RwLockWriteGuard<'a, State>,
    ) -> Result<RwLockWriteGuard<'a, State>> {
        // Apply cache
        let ponds = &mut state.ponds;
        let now = Utc::now();
        if now < ponds.created_at + self.pond_ttl {
            return Ok(state);
        }

        // Load all available pond service endpoints
        let uris = self.discover().await?;

        // Query to each pond and get available devices
        let data = uris
            .into_iter()
            .map(|uri| async move {
                let mut client = match Channel::builder(uri.clone()).connect().await {
                    Ok(channel) => PondClient::new(channel),
                    Err(error) => {
                        return Err(Status::unavailable(format!(
                            "Failed to connect to pond {uri:?}: {error}"
                        )));
                    }
                };

                let pond::ListDevicesResponse {
                    id,
                    devices,
                    topology,
                } = client
                    .list_devices(pond::ListDevicesRequest {})
                    .await?
                    .into_inner();

                let pond = Pond {
                    client: Mutex::new(client),
                    devices,
                    id: id.clone(),
                    topology: topology.unwrap_or_default(),
                };
                Result::<_, Status>::Ok((id, Arc::new(pond)))
            })
            .collect::<FuturesUnordered<_>>()
            .try_collect()
            .await?;

        // Store outputs
        *ponds = Ponds {
            created_at: now,
            data,
        };
        Ok(state)
    }
}

#[async_trait]
impl Controller for Server {
    async fn create_volume(
        &self,
        request: Request<csi::CreateVolumeRequest>,
    ) -> Result<Response<csi::CreateVolumeResponse>> {
        // ****************************************
        // Step 0: Validate inputs
        // ****************************************

        let request = request.into_inner();
        #[cfg(feature = "tracing")]
        debug!("request = {request:#?}");

        let csi::CreateVolumeRequest {
            name,
            capacity_range,
            volume_capabilities,
            parameters,
            secrets,
            volume_content_source,
            accessibility_requirements,
            mutable_parameters,
        } = request;

        // Define volume ID
        let volume_id = format!("pond_{name}");

        // Validate capacity
        let csi::CapacityRange {
            required_bytes,
            limit_bytes: _,
        } = capacity_range.ok_or_else(|| Status::invalid_argument("missing capacity"))?;

        // ****************************************
        // Step 1: Validate volume options
        // ****************************************

        let mut options = self.default.volume_options();
        options.apply_volume_capabilities(volume_capabilities)?;

        // ****************************************
        // Step 2: Validate volume parameters
        // ****************************************

        let attributes: VolumeAttributes = vec![parameters, mutable_parameters].parse()?;
        let secrets: VolumeSecrets = secrets.parse()?;

        // ****************************************
        // Step 3: Build device filters
        // ****************************************

        // Validate volume content source
        let content_source = match volume_content_source {
            Some(_) => {
                return Err(Status::unimplemented(
                    "create_volume::volume_content_source is not supported yet",
                ));
            }
            None => None,
        };

        // Filter accessible topology
        struct TopologyRequirement {
            requisite: bool,
            preferred: usize,
        }
        let filter_topology = move |pond: &Pond| {
            let topology = &pond.topology.provides;
            match accessibility_requirements.as_ref() {
                Some(csi::TopologyRequirement {
                    requisite,
                    preferred,
                }) => TopologyRequirement {
                    // ANY of topologies
                    requisite: requisite.iter().any(|csi::Topology { segments }| {
                        segments
                            .iter()
                            .all(|(key, value)| topology.get(key) == Some(value))
                    }),
                    // COUNT of topologies
                    preferred: preferred
                        .iter()
                        .filter(|csi::Topology { segments }| {
                            segments
                                .iter()
                                .all(|(key, value)| topology.get(key) == Some(value))
                        })
                        .count(),
                },
                None => TopologyRequirement {
                    requisite: true,
                    preferred: 0,
                },
            }
        };

        // ****************************************
        // Step 4: [C] Filter devices
        // ****************************************

        // Load ponds
        let state = self.state.write().await;
        let mut state = self.fetch_ponds(state).await?;

        // Filter devices
        let available_devices: BTreeMap<_, _> = state
            .ponds
            .data
            .iter()
            .flat_map(|(pond_id, pond)| {
                pond.devices.iter().cloned().map(|data| {
                    let key = DeviceKey {
                        pond_id: pond_id.clone(),
                        device_id: data.id.clone(),
                    };
                    let offset = state.allocated.get(&key).copied().unwrap_or_default();
                    Volume {
                        attributes: attributes.clone(),
                        data,
                        offset,
                        pond: pond.clone(),
                        reserved: 0,
                        volume_id: volume_id.clone(),
                    }
                })
            })
            .filter_map(move |device| {
                // Apply topology
                let TopologyRequirement {
                    requisite,
                    preferred,
                } = filter_topology(&device.pond);

                if !requisite {
                    return None;
                }

                let capacity = device.available();
                let priority = (
                    !preferred,
                    -capacity,
                    device.pond.id.clone(),
                    device.data.id.clone(),
                );
                Some((priority, device))
            })
            .collect();

        // Replicate the volume
        let num_replicas = attributes
            .num_replicas
            .as_ref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1)
            .max(1);

        // Pick up devices
        let mut devices = Vec::default();
        {
            let mut available = available_devices.into_values();
            let padding = attributes.layer.margin();
            let required = required_bytes + padding;
            let mut total_filled = 0;
            let total_required = required * num_replicas;
            for _ in 0..num_replicas {
                let mut filled = 0;
                while let Some(mut device) = available.next() {
                    let remaining = required - filled;
                    let reserved = device.reserve(remaining);
                    devices.push(device);
                    filled += reserved;
                    total_filled += reserved;
                    if filled >= required {
                        break;
                    }
                }
            }
            if total_filled < total_required {
                return Err(Status::resource_exhausted(format!(
                    "Out Of Capacity: expected {total_required} bytes but given {total_filled} bytes"
                )));
            }
        }

        // ****************************************
        // Step 5: [C, E] Create a VG
        // ****************************************

        // Build accessible topology
        let topology_segments = devices
            .iter()
            .map(|device| device.pond.topology.provides.clone())
            .fold(HashMap::default(), |mut acc, x| {
                acc.extend(x);
                acc
            });

        // Build volume context
        let volume_context = ::serde_json::to_value(&attributes)
            .and_then(::serde_json::from_value)
            // conceal secrets
            .map_err(|_| Status::internal("Failed to build volume attributes"))?;

        // Define a new VG
        let vg = VolumeGroup {
            data: csi::Volume {
                capacity_bytes: required_bytes,
                volume_id,
                volume_context,
                content_source,
                accessible_topology: if topology_segments.is_empty() {
                    Default::default()
                } else {
                    vec![csi::Topology {
                        segments: topology_segments,
                    }]
                },
            },
            devices,
            options,
        };

        // Claim LV
        let volume = {
            let pv = vg.allocate(&secrets).await?;
            let volume = pv.group.data.clone();
            for device in &pv.group.devices {
                let key = DeviceKey {
                    pond_id: device.pond.id.clone(),
                    device_id: device.data.id.clone(),
                };
                *state.allocated.entry(key).or_default() += device.reserved;
            }
            state.volumes.insert(pv.group.data.volume_id.clone(), pv);
            drop(state); // release lock
            volume
        };
        Ok(Response::new(csi::CreateVolumeResponse {
            volume: Some(volume),
        }))
    }

    async fn delete_volume(
        &self,
        request: Request<csi::DeleteVolumeRequest>,
    ) -> Result<Response<csi::DeleteVolumeResponse>> {
        // ****************************************
        // Step 0: Validate inputs
        // ****************************************

        let request = request.into_inner();
        #[cfg(feature = "tracing")]
        debug!("request = {request:#?}");

        let csi::DeleteVolumeRequest { volume_id, secrets } = request;

        // ****************************************
        // Step 1: Validate volume parameters
        // ****************************************

        let secrets: VolumeSecrets = secrets.parse()?;

        // ****************************************
        // Step 2: [C] Validate volume
        // ****************************************

        // Find the volume
        let mut state = self.state.write().await;
        let PersistentVolume {
            condition: csi::VolumeCondition { abnormal, message },
            group,
            published_node_ids,
            readonly: _,
        } = {
            match state.volumes.get(&volume_id) {
                Some(volume) => volume,
                None => return Ok(Response::new(csi::DeleteVolumeResponse {})),
            }
        };

        // Stop if the volume is abnormal
        if *abnormal {
            return Err(Status::aborted(message.clone()));
        }

        // Stop if the published nodes exist
        if !published_node_ids.is_empty() {
            return Err(Status::aborted(format!(
                "Published volume: {volume_id} -> {published_node_ids:?}"
            )));
        }

        // ****************************************
        // Step 3: [C, E] Execute
        // ****************************************

        // Release devices
        group.deallocate(&secrets).await?;

        // Remove volume
        {
            for device in group.devices.clone() {
                let key = DeviceKey {
                    pond_id: device.pond.id.clone(),
                    device_id: device.data.id.clone(),
                };
                if let Some(allocated) = state.allocated.get_mut(&key) {
                    *allocated -= device.reserved;
                }
            }
            state.volumes.remove(&volume_id);
            drop(state);
        }
        Ok(Response::new(csi::DeleteVolumeResponse {}))
    }

    async fn controller_publish_volume(
        &self,
        request: Request<csi::ControllerPublishVolumeRequest>,
    ) -> Result<Response<csi::ControllerPublishVolumeResponse>> {
        // ****************************************
        // Step 0: Validate inputs
        // ****************************************

        let request = request.into_inner();
        #[cfg(feature = "tracing")]
        debug!("request = {request:#?}");

        let csi::ControllerPublishVolumeRequest {
            volume_id,
            node_id,
            volume_capability,
            readonly,
            secrets,
            volume_context,
        } = request;

        // ****************************************
        // Step 1: Validate volume options
        // ****************************************

        let mut options = self.default.volume_options();
        options.apply_volume_capability(volume_capability.unwrap_or_default())?;
        drop(options);

        // ****************************************
        // Step 2: Validate volume parameters
        // ****************************************

        let _: VolumeAttributes = volume_context.parse()?;
        let _: VolumeSecrets = secrets.parse()?;

        // ****************************************
        // Step 3: [C] Validate volume
        // ****************************************

        // Find the volume
        let mut state = self.state.write().await;
        let PersistentVolume {
            condition: csi::VolumeCondition { abnormal, message },
            group,
            published_node_ids,
            readonly: group_readonly,
        } = {
            match state.volumes.get_mut(&volume_id) {
                Some(volume) => volume,
                None => return Err(Status::not_found(format!("No such volume: {volume_id}"))),
            }
        };

        // Stop if the volume is abnormal
        if *abnormal {
            return Err(Status::aborted(message.clone()));
        }

        // Stop if the volume group is not shared and one of the nodes has been published as write mode.
        if !group.options.mount_shared {
            let has_published = !published_node_ids.is_empty();
            let group_readonly = *group_readonly;
            if !readonly && group_readonly && has_published {
                return Err(Status::aborted(format!(
                    "Already published volume AS ro: {volume_id}"
                )));
            }
            if readonly && !group_readonly && has_published {
                return Err(Status::aborted(format!(
                    "Already published volume AS rw: {volume_id}"
                )));
            }
        }

        // ****************************************
        // Step 4: [C] Publish node
        // ****************************************

        let publish_context = {
            let context = VolumePublishControllerContext {
                devices: group
                    .devices
                    .iter()
                    .map(|device| device.data.clone())
                    .collect(),
            }
            .to_dict()?;

            published_node_ids.insert(node_id);
            *group_readonly |= readonly;
            drop(state);
            context
        };

        Ok(Response::new(csi::ControllerPublishVolumeResponse {
            publish_context,
        }))
    }

    async fn controller_unpublish_volume(
        &self,
        request: Request<csi::ControllerUnpublishVolumeRequest>,
    ) -> Result<Response<csi::ControllerUnpublishVolumeResponse>> {
        // ****************************************
        // Step 0: Validate inputs
        // ****************************************

        let request = request.into_inner();
        #[cfg(feature = "tracing")]
        debug!("request = {request:#?}");

        let csi::ControllerUnpublishVolumeRequest {
            volume_id,
            node_id,
            secrets,
        } = request;

        // ****************************************
        // Step 1: Validate volume parameters
        // ****************************************

        let _: VolumeSecrets = secrets.parse()?;

        // ****************************************
        // Step 2: [C] Validate volume
        // ****************************************

        // Find the volume
        let mut state = self.state.write().await;
        let PersistentVolume {
            condition: csi::VolumeCondition { abnormal, message },
            group: _,
            published_node_ids,
            readonly,
        } = {
            match state.volumes.get_mut(&volume_id) {
                Some(volume) => volume,
                None => return Ok(Response::new(csi::ControllerUnpublishVolumeResponse {})),
            }
        };

        // Stop if the volume is abnormal
        if *abnormal {
            return Err(Status::aborted(message.clone()));
        }

        // ****************************************
        // Step 3: [C] Unpublish node
        // ****************************************

        {
            published_node_ids.remove(&node_id);
            *readonly &= !published_node_ids.is_empty();
            drop(state);
        }

        Ok(Response::new(csi::ControllerUnpublishVolumeResponse {}))
    }

    async fn validate_volume_capabilities(
        &self,
        request: Request<csi::ValidateVolumeCapabilitiesRequest>,
    ) -> Result<Response<csi::ValidateVolumeCapabilitiesResponse>> {
        #[cfg(feature = "tracing")]
        warn!("request = {:#?}", request.into_inner());
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
            .read()
            .await
            .volumes
            .range(starting_token..)
            .take(max_entries + 1)
            .map(|(_, pv)| csi::list_volumes_response::Entry {
                volume: Some(pv.group.data.clone()),
                status: Some(csi::list_volumes_response::VolumeStatus {
                    published_node_ids: pv.published_node_ids.iter().cloned().collect(),
                    volume_condition: Some(pv.condition.clone()),
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
        #[cfg(feature = "tracing")]
        warn!("request = {:#?}", request.into_inner());
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
                                r#type: csi::controller_service_capability::rpc::Type::PublishUnpublishVolume as _,
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
                    // csi::ControllerServiceCapability {
                    //     r#type: Some(csi::controller_service_capability::Type::Rpc(
                    //         csi::controller_service_capability::Rpc {
                    //             r#type: csi::controller_service_capability::rpc::Type::GetCapacity as _,
                    //         },
                    //     )),
                    // },
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
                                r#type: csi::controller_service_capability::rpc::Type::ListVolumesPublishedNodes as _,
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
                                r#type: csi::controller_service_capability::rpc::Type::SingleNodeMultiWriter as _,
                            },
                        )),
                    },
                    // csi::ControllerServiceCapability {
                    //     r#type: Some(csi::controller_service_capability::Type::Rpc(
                    //         csi::controller_service_capability::Rpc {
                    //             r#type: csi::controller_service_capability::rpc::Type::ModifyVolume as _,
                    //         },
                    //     )),
                    // },
                ],
            },
        ))
    }

    async fn create_snapshot(
        &self,
        request: Request<csi::CreateSnapshotRequest>,
    ) -> Result<Response<csi::CreateSnapshotResponse>> {
        #[cfg(feature = "tracing")]
        warn!("request = {:#?}", request.into_inner());
        crate::todo!("create_snapshot")
    }

    async fn delete_snapshot(
        &self,
        request: Request<csi::DeleteSnapshotRequest>,
    ) -> Result<Response<csi::DeleteSnapshotResponse>> {
        #[cfg(feature = "tracing")]
        warn!("request = {:#?}", request.into_inner());
        crate::todo!("delete_snapshot")
    }

    async fn list_snapshots(
        &self,
        request: Request<csi::ListSnapshotsRequest>,
    ) -> Result<Response<csi::ListSnapshotsResponse>> {
        #[cfg(feature = "tracing")]
        warn!("request = {:#?}", request.into_inner());
        crate::todo!("list_snapshots")
    }

    async fn get_snapshot(
        &self,
        request: Request<csi::GetSnapshotRequest>,
    ) -> Result<Response<csi::GetSnapshotResponse>> {
        #[cfg(feature = "tracing")]
        warn!("request = {:#?}", request.into_inner());
        crate::todo!("get_snapshot")
    }

    async fn controller_expand_volume(
        &self,
        request: Request<csi::ControllerExpandVolumeRequest>,
    ) -> Result<Response<csi::ControllerExpandVolumeResponse>> {
        // FIXME: To be implemented!
        #[cfg(feature = "tracing")]
        warn!("request = {:#?}", request.into_inner());
        crate::todo!("controller_expand_volume")
    }

    async fn controller_get_volume(
        &self,
        request: Request<csi::ControllerGetVolumeRequest>,
    ) -> Result<Response<csi::ControllerGetVolumeResponse>> {
        let csi::ControllerGetVolumeRequest { volume_id } = request.into_inner();

        match self.state.read().await.volumes.get(&volume_id) {
            Some(pv) => Ok(Response::new(csi::ControllerGetVolumeResponse {
                volume: Some(pv.group.data.clone()),
                status: Some(csi::controller_get_volume_response::VolumeStatus {
                    published_node_ids: pv.published_node_ids.iter().cloned().collect(),
                    volume_condition: Some(pv.condition.clone()),
                }),
            })),
            None => Err(Status::not_found(format!("No such volume: {volume_id}"))),
        }
    }

    async fn controller_modify_volume(
        &self,
        request: Request<csi::ControllerModifyVolumeRequest>,
    ) -> Result<Response<csi::ControllerModifyVolumeResponse>> {
        #[cfg(feature = "tracing")]
        warn!("request = {:#?}", request.into_inner());
        crate::todo!("controller_modify_volume")
    }
}
