use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::Arc,
};

use anyhow::Error;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use data_pond_api::{
    VolumeAttributes, VolumeBindingContext, VolumePublishControllerContext, VolumeSecrets,
};
use data_pond_csi::{
    csi::{self, controller_server::Controller},
    pond::{self, pond_client::PondClient},
};
use futures::{
    TryStreamExt,
    stream::{FuturesOrdered, FuturesUnordered},
};
use hickory_resolver::{
    Resolver,
    name_server::{GenericConnector, TokioConnectionProvider},
    proto::runtime::TokioRuntimeProvider,
};
use tokio::sync::{Mutex, MutexGuard};
use tonic::{
    Request, Response, Result, Status,
    transport::{Channel, Uri},
};
#[cfg(feature = "tracing")]
use tracing::{debug, warn};

use crate::volume::{
    Pond, VolumeBindingClaim, VolumeOptionsExt, VolumeParametersExport, VolumeParametersSource,
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

#[derive(Debug)]
struct VolumeClaim {
    bindings: Vec<VolumeBindingClaim>,
    options: pond::VolumeOptions,
}

impl VolumeClaim {
    async fn allocate(mut self, secrets: &VolumeSecrets) -> Result<Volume> {
        let Self { bindings, options } = &mut self;

        // Allocate volumes
        bindings
            .iter_mut()
            .map(|binding| binding.allocate(secrets.clone(), options.clone()))
            .collect::<FuturesOrdered<_>>()
            .try_collect::<()>()
            .await?;

        Ok(Volume {
            claim: self,
            published_node_ids: Default::default(),
            readonly: false,
        })
    }

    async fn deallocate(&self, secrets: &VolumeSecrets) -> Result<()> {
        let Self { bindings, options } = self;

        // Deallocate volumes
        bindings
            .into_iter()
            .map(|binding| binding.deallocate(secrets.clone(), options.clone()))
            .collect::<FuturesOrdered<_>>()
            .try_collect::<()>()
            .await?;

        Ok(())
    }
}

#[derive(Debug)]
struct Volume {
    claim: VolumeClaim,
    published_node_ids: BTreeSet<String>,
    readonly: bool,
}

#[derive(Debug, Default)]
struct State {
    allocated: HashMap<DeviceKey, i64>,
    ponds: Ponds,
    volumes: BTreeMap<String, Volume>,
}

impl State {
    fn get_volume(&mut self, volume_id: &str) -> Result<&mut Volume> {
        // Use cached volume
        self.volumes
            .get_mut(volume_id)
            .ok_or_else(|| Status::not_found(format!("No such volume: {volume_id}")))
    }

    async fn get_volume_or_provision(
        &mut self,
        volume_id: &str,
        options: pond::VolumeOptions,
        attributes: &VolumeAttributes,
    ) -> Result<&mut Volume> {
        if !self.volumes.contains_key(volume_id) {
            // Discover the volume bindings
            let mut bindings = Vec::default();
            for (pond_id, pond) in &self.ponds.data {
                for metadata in pond
                    .bindings
                    .iter()
                    .filter(|&metadata| metadata.volume_id == volume_id)
                {
                    let device_id = &metadata.device_id;
                    bindings.push(VolumeBindingClaim {
                        attributes: attributes.clone(),
                        device: pond
                            .devices
                            .iter()
                            .find(|&d| d.id == *device_id)
                            .cloned()
                            .ok_or_else(|| {
                                Status::internal(format!(
                                    "No such device: {pond_id} -> {device_id}"
                                ))
                            })?,
                        metadata: metadata.clone(),
                        pond: pond.clone(),
                    });
                }
            }

            // Validate the bindings
            if bindings.is_empty() {
                return Err(Status::unavailable("Volume bindings not found"));
            }
            {
                let num_bindings = bindings.len();
                let total_bindings: usize = bindings[0].metadata.total_bindings as _;
                if num_bindings != total_bindings {
                    return Err(Status::unavailable(format!(
                        "Insufficient volume bindings: expected {total_bindings}, but found {num_bindings}"
                    )));
                }
            }
            {
                bindings.sort_by_key(|p| p.metadata.index_bindings);
                if bindings
                    .iter()
                    .enumerate()
                    .any(|(index, p)| index != p.metadata.index_bindings as usize)
                {
                    return Err(Status::internal("Invalid volume partition order"));
                }
            }

            // Store the volume
            let volume = Volume {
                claim: VolumeClaim { bindings, options },
                published_node_ids: Default::default(),
                readonly: false,
            };
            self.volumes.insert(volume_id.into(), volume);
        }

        Ok(self.volumes.get_mut(volume_id).unwrap())
    }
}

pub(crate) struct Server {
    default: super::DefaultSettings,
    pond_host: String,
    pond_port: u16,
    pond_protocol: String,
    pond_ttl: Duration,
    resolver: Resolver<GenericConnector<TokioRuntimeProvider>>,
    state: Mutex<State>,
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
            let _ = server.fetch_ponds().await?;
        };
        Ok(server)
    }

    async fn discover_ponds(&self) -> Result<Vec<Uri>> {
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

    async fn fetch_ponds(&self) -> Result<MutexGuard<'_, State>> {
        // Apply cache
        let mut state = self.state.lock().await;
        let ponds = &mut state.ponds;
        let now = Utc::now();
        if now < ponds.created_at + self.pond_ttl {
            return Ok(state);
        }

        // Load all available pond service endpoints
        let uris = self.discover_ponds().await?;

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
                    bindings,
                    id,
                    devices,
                    topology,
                } = client
                    .list_devices(pond::ListDevicesRequest {})
                    .await?
                    .into_inner();

                let pond = Pond {
                    bindings,
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
        let mut state = self.fetch_ponds().await?;

        // Filter devices
        let available_bindings: BTreeMap<_, _> = state
            .ponds
            .data
            .iter()
            .flat_map(|(pond_id, pond)| {
                pond.devices.iter().cloned().map(|device| {
                    let key = DeviceKey {
                        pond_id: pond_id.clone(),
                        device_id: device.id.clone(),
                    };
                    let offset = state.allocated.get(&key).copied().unwrap_or_default();
                    VolumeBindingClaim {
                        attributes: attributes.clone(),
                        device,
                        metadata: pond::VolumeBindingMetadata {
                            device_id: key.device_id,
                            index_bindings: 0,
                            offset,
                            reserved: 0,
                            total_bindings: 0,
                            volume_id: volume_id.clone(),
                        },
                        pond: pond.clone(),
                    }
                })
            })
            .filter_map(move |binding| {
                // Apply topology
                let TopologyRequirement {
                    requisite,
                    preferred,
                } = filter_topology(&binding.pond);

                if !requisite {
                    return None;
                }

                let capacity = binding.available();
                let priority = (
                    !preferred,
                    -capacity,
                    binding.pond.id.clone(),
                    binding.device.id.clone(),
                );
                Some((priority, binding))
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
        let mut bindings = Vec::default();
        {
            let mut available = available_bindings.into_values();
            let padding = attributes.layer.margin();
            let required = required_bytes + padding;
            let mut total_filled = 0;
            let total_required = required * num_replicas;
            for _ in 0..num_replicas {
                let mut filled = 0;
                while let Some(mut binding) = available.next() {
                    let remaining = required - filled;
                    let reserved = binding.reserve(remaining);
                    bindings.push(binding);
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

        // Store the binding order
        let total_bindings = bindings.len() as _;
        for (index, binding) in bindings.iter_mut().enumerate() {
            binding.metadata.index_bindings = index as _;
            binding.metadata.total_bindings = total_bindings;
        }

        // ****************************************
        // Step 5: [C, E] Create a volume claim
        // ****************************************

        // Build accessible topology
        let topology_segments = bindings
            .iter()
            .map(|binding| binding.pond.topology.provides.clone())
            .fold(HashMap::default(), |mut acc, x| {
                acc.extend(x);
                acc
            });

        // Build volume context
        let volume_context = ::serde_json::to_value(&attributes)
            .and_then(::serde_json::from_value)
            // conceal secrets
            .map_err(|_| Status::internal("Failed to build volume attributes"))?;

        // Define a new volume claim
        let claim = VolumeClaim { bindings, options };
        let data = csi::Volume {
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
        };

        // Execute a claim
        {
            let volume = claim.allocate(&secrets).await?;
            for binding in &volume.claim.bindings {
                let key = DeviceKey {
                    pond_id: binding.pond.id.clone(),
                    device_id: binding.device.id.clone(),
                };
                *state.allocated.entry(key).or_default() += binding.metadata.reserved;
            }
            state.volumes.insert(data.volume_id.clone(), volume);
            drop(state); // release lock
        };
        Ok(Response::new(csi::CreateVolumeResponse {
            volume: Some(data),
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
        let mut state = self.fetch_ponds().await?;
        let volume = state.get_volume(&volume_id)?;

        // Stop if the published nodes exist
        let published_node_ids = &volume.published_node_ids;
        if !published_node_ids.is_empty() {
            return Err(Status::aborted(format!(
                "Published volume: {volume_id} -> {published_node_ids:?}"
            )));
        }

        // ****************************************
        // Step 3: [C, E] Execute
        // ****************************************

        // Release devices
        volume.claim.deallocate(&secrets).await?;

        // Remove volume
        {
            for binding in volume.claim.bindings.clone() {
                let key = DeviceKey {
                    pond_id: binding.pond.id.clone(),
                    device_id: binding.device.id.clone(),
                };
                if let Some(allocated) = state.allocated.get_mut(&key) {
                    *allocated -= binding.metadata.reserved;
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

        // ****************************************
        // Step 2: Validate volume parameters
        // ****************************************

        let attributes: VolumeAttributes = volume_context.parse()?;
        let _: VolumeSecrets = secrets.parse()?;

        // ****************************************
        // Step 3: [C, E] Validate volume
        // ****************************************

        // Find the volume
        let mut state = self.fetch_ponds().await?;
        let volume = state
            .get_volume_or_provision(&volume_id, options, &attributes)
            .await?;

        // Stop if the volume group is not shared and one of the nodes has been published as write mode.
        let published_node_ids = &mut volume.published_node_ids;
        let group_readonly = &mut volume.readonly;
        if !volume.claim.options.mount_shared {
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
                bindings: volume
                    .claim
                    .bindings
                    .iter()
                    .map(|binding| VolumeBindingContext {
                        device: binding.device.clone(),
                        layer: binding.device.layer(),
                        metadata: binding.metadata.clone(),
                        source: binding.device.source(),
                    })
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
        let mut state = self.fetch_ponds().await?;
        let volume = state.get_volume(&volume_id)?;

        // ****************************************
        // Step 3: [C] Unpublish node
        // ****************************************

        {
            let published_node_ids = &mut volume.published_node_ids;
            published_node_ids.remove(&node_id);
            volume.readonly &= !published_node_ids.is_empty();
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
        #[cfg(feature = "tracing")]
        warn!("request = {:#?}", request.into_inner());
        crate::todo!("list_volumes")
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
        #[cfg(feature = "tracing")]
        warn!("request = {:#?}", request.into_inner());
        crate::todo!("controller_get_volume")
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
