use std::collections::{BTreeMap, BTreeSet, HashMap};

use anyhow::Error;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use data_pond_api::VolumeParameters;
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
use serde_json::{Map, Value};
use strum::{Display, EnumString};
use tokio::sync::{Mutex, MutexGuard, RwLock};
use tonic::{
    Request, Response, Result, Status,
    transport::{Channel, Uri},
};
#[cfg(feature = "tracing")]
use tracing::debug;

#[derive(
    Copy, Clone, Debug, Display, Default, EnumString, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[strum(serialize_all = "kebab-case")]
enum FsType {
    #[default]
    Ext4,
}

#[derive(Clone, Debug)]
struct Pond {
    client: PondClient<Channel>,
    devices: Vec<pond::Device>,
    topology: pond::DeviceTopology,
}

impl Pond {
    fn capacity(&self) -> i64 {
        self.devices.iter().map(|device| device.capacity).sum()
    }
}

#[derive(Clone, Debug, Default)]
struct Ponds {
    created_at: DateTime<Utc>,
    data: BTreeMap<String, Pond>,
}

impl Ponds {
    fn capacity(&self) -> i64 {
        self.data.values().map(|pond| pond.capacity()).sum()
    }
}

#[derive(Debug)]
struct VolumeDevice<'a> {
    data: &'a pond::Device,
    endpoint: &'a str,
    pond: &'a Pond,
}

impl VolumeDevice<'_> {
    fn available(&self) -> i64 {
        self.capacity() - self.padding()
    }

    #[inline]
    const fn capacity(&self) -> i64 {
        self.data.capacity
    }

    fn padding(&self) -> i64 {
        self.data.layer().padding()
    }
}

#[derive(Debug)]
struct VolumeGroup<'a> {
    data: csi::Volume,
    devices: Vec<VolumeDevice<'a>>,
    fs_type: Option<FsType>,
    layer: pond::device_layer::Type,
    mount_flags: Vec<String>,
    mount_group: String,
    mount_shared: bool,
    num_replicas: i64,
    published_node_ids: BTreeSet<String>,
}

impl VolumeGroup<'_> {
    async fn claim(self) -> Result<PersistentVolume> {
        dbg!(&self);
        Ok(PersistentVolume {
            data: self.data.clone(),
            condition: csi::VolumeCondition {
                abnormal: false,
                message: "Provisioned".into(),
            },
            published_node_ids: self.published_node_ids.clone(),
        })
    }
}

struct PersistentVolume {
    data: csi::Volume,
    condition: csi::VolumeCondition,
    published_node_ids: BTreeSet<String>,
}

pub(crate) struct Server {
    pond_host: String,
    pond_port: u16,
    pond_protocol: String,
    pond_ttl: Duration,
    ponds: Mutex<Ponds>,
    resolver: Resolver<GenericConnector<TokioRuntimeProvider>>,
    volumes: RwLock<BTreeMap<String, PersistentVolume>>,
}

impl Server {
    pub(crate) async fn try_new() -> Result<Self, Error> {
        // Construct a new Resolver with default configuration options
        let provider = TokioConnectionProvider::default();
        let resolver = Resolver::builder(provider)?.build();

        // Initialize ponds
        let server = Self {
            ponds: Default::default(),
            pond_host: "plugin.hoya.svc.ops.openark".into(),
            pond_port: 9090,
            pond_protocol: "http".into(),
            pond_ttl: Duration::seconds(30),
            resolver,
            volumes: Default::default(),
        };
        let _ = server.fetch_ponds().await?;
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

    async fn fetch_ponds(&self) -> Result<MutexGuard<'_, Ponds>> {
        // Apply cache
        let mut ponds = self.ponds.lock().await;
        let now = Utc::now();
        if now < ponds.created_at + self.pond_ttl {
            return Ok(ponds);
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

                let pond::ListDevicesResponse { devices, topology } = client
                    .list_devices(pond::ListDevicesRequest {})
                    .await?
                    .into_inner();

                let endpoint = Pond {
                    client,
                    devices,
                    topology: topology.unwrap_or_default(),
                };
                Result::<_, Status>::Ok((uri.to_string(), endpoint))
            })
            .collect::<FuturesUnordered<_>>()
            .try_collect()
            .await?;

        // Store outputs
        *ponds = Ponds {
            created_at: now,
            data,
        };
        Ok(ponds)
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

        // Validate capacity
        let csi::CapacityRange {
            required_bytes,
            limit_bytes: _,
        } = capacity_range.ok_or_else(|| Status::invalid_argument("missing capacity"))?;

        // ****************************************
        // Step 1: Detect volume capabilities
        // ****************************************

        let mut fs_type = Some(FsType::Ext4);
        let mut mount_flags = Vec::default();
        let mut mount_group = String::default();
        let mut mount_shared = false;
        for csi::VolumeCapability {
            access_mode,
            access_type,
        } in volume_capabilities
        {
            // Validate access mode
            match access_mode.map(|field| field.mode()) {
                Some(
                    csi::volume_capability::access_mode::Mode::MultiNodeMultiWriter
                    | csi::volume_capability::access_mode::Mode::MultiNodeReaderOnly
                    | csi::volume_capability::access_mode::Mode::MultiNodeSingleWriter,
                ) => mount_shared = true,
                Some(
                    csi::volume_capability::access_mode::Mode::SingleNodeMultiWriter
                    | csi::volume_capability::access_mode::Mode::SingleNodeReaderOnly
                    | csi::volume_capability::access_mode::Mode::SingleNodeSingleWriter
                    | csi::volume_capability::access_mode::Mode::SingleNodeWriter,
                ) => (),
                Some(csi::volume_capability::access_mode::Mode::Unknown) | None => (),
            }

            // Validate access type
            match access_type {
                Some(csi::volume_capability::AccessType::Block(
                    csi::volume_capability::BlockVolume {},
                )) => {
                    // Validate access mode
                    if mount_shared {
                        return Err(Status::invalid_argument(
                            "Multi node PVC is not supported as a block device access mode",
                        ));
                    }
                    fs_type = None;
                    mount_flags.clear();
                    mount_group.clear();
                }
                Some(csi::volume_capability::AccessType::Mount(m)) => {
                    // Validate filesystem type
                    if !m.fs_type.is_empty() {
                        fs_type = Some(m.fs_type.parse().map_err(|_| {
                            Status::invalid_argument(format!(
                                "Unsupported filesystem type: {}",
                                &m.fs_type,
                            ))
                        })?);
                    };
                    mount_flags = m.mount_flags;
                    mount_group = m.volume_mount_group;
                }
                None => (),
            }
        }

        // ****************************************
        // Step 2: Build device filters
        // ****************************************

        // Validate parameters (parameters -> mutable parameters -> secrets)
        let VolumeParameters { attributes } = {
            let mut map = Map::default();
            {
                let mut extend = |params: HashMap<String, String>| {
                    map.extend(
                        params
                            .into_iter()
                            .map(|(key, value)| (key, Value::String(value))),
                    )
                };
                extend(parameters);
                extend(mutable_parameters);
                extend(secrets);
            }
            ::serde_json::from_value(Value::Object(map))
                // conceal secrets
                .map_err(|_| Status::invalid_argument("Invalid parameters or secrets"))?
        };

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
        // Step 3: [C] Filter devices
        // ****************************************

        // Load ponds
        let ponds = self.fetch_ponds().await?;

        // Filter devices
        let available_devices: BTreeMap<_, _> = ponds
            .data
            .iter()
            .flat_map(|(endpoint, pond)| {
                pond.devices.iter().map(move |data| VolumeDevice {
                    data,
                    endpoint: endpoint.as_str(),
                    pond,
                })
            })
            .filter_map(move |device| {
                // Apply topology
                let TopologyRequirement {
                    requisite,
                    preferred,
                } = filter_topology(device.pond);

                if !requisite {
                    return None;
                }

                let capacity = device.capacity();
                let priority = (!preferred, -capacity, device.endpoint);
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

        // Select target layer
        let layer = pond::device_layer::Type::Lvm;

        // Pick up devices
        let mut devices = Vec::default();
        {
            let available = available_devices.into_values();
            let mut filled = 0;
            let padding = layer.margin();
            let required = (required_bytes + padding) * num_replicas;
            for device in available {
                let capacity = device.available();
                devices.push(device);
                filled += capacity;
                if filled >= required {
                    break;
                }
            }
            if filled < required {
                return Err(Status::resource_exhausted(format!(
                    "Out Of Capacity: expected {required} bytes but given {filled} bytes"
                )));
            }
        }

        // ****************************************
        // Step 4: [C] Create a VG
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
                volume_id: format!("pond_{name}"),
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
            fs_type,
            mount_flags,
            mount_group,
            mount_shared,
            layer,
            num_replicas,
            published_node_ids: Default::default(),
        };

        // Claim LV
        let volume = {
            let mut volumes = self.volumes.write().await;
            let pv = vg.claim().await?;
            let volume = pv.data.clone();
            volumes.insert(pv.data.volume_id.clone(), pv);
            drop(ponds); // release ponds lock
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
