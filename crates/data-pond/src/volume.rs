use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
};

use data_pond_api::{VolumeAttributes, VolumePublishControllerContext, VolumeSecrets};
use data_pond_csi::{
    csi,
    pond::{self, pond_client::PondClient},
};
use futures::{TryStreamExt, stream::FuturesOrdered};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Map, Value};
use tokio::sync::Mutex;
use tonic::{Result, Status, transport::Channel};

#[derive(Debug)]
pub(crate) struct Pond {
    pub(crate) client: Mutex<PondClient<Channel>>,
    pub(crate) devices: Vec<pond::Device>,
    pub(crate) id: String,
    pub(crate) topology: pond::DeviceTopology,
}

#[derive(Clone, Debug)]
pub(crate) struct Volume {
    pub(crate) attributes: VolumeAttributes,
    pub(crate) data: pond::Device,
    pub(crate) offset: i64,
    pub(crate) pond: Arc<Pond>,
    pub(crate) reserved: i64,
    pub(crate) volume_id: String,
}

impl Volume {
    #[inline]
    pub(crate) fn available(&self) -> i64 {
        self.data.capacity - self.offset - self.padding()
    }

    #[inline]
    fn padding(&self) -> i64 {
        self.data.layer().padding()
    }

    pub(crate) fn reserve(&mut self, remaining: i64) -> i64 {
        let available = self.available();
        let reserved = available.min(remaining);
        self.reserved = reserved + self.padding();
        reserved
    }

    fn build(
        &self,
        secrets: VolumeSecrets,
        options: pond::VolumeOptions,
    ) -> Result<pond::AllocateVolumeRequest> {
        Ok(pond::AllocateVolumeRequest {
            device_id: self.data.id.clone(),
            volume_id: self.volume_id.clone(),
            capacity: self.reserved,
            options: Some(options),
            attributes: self.attributes.to_dict()?,
            secrets: secrets.to_dict()?,
        })
    }

    async fn allocate(self, secrets: VolumeSecrets, options: pond::VolumeOptions) -> Result<Self> {
        let request = self.build(secrets, options)?;
        let pond::AllocateVolumeResponse {} = self
            .pond
            .client
            .lock()
            .await
            .allocate_volume(request)
            .await?
            .into_inner();
        Ok(self)
    }

    async fn deallocate(&self, secrets: VolumeSecrets, options: pond::VolumeOptions) -> Result<()> {
        let request = self.build(secrets, options)?;
        let pond::AllocateVolumeResponse {} = self
            .pond
            .client
            .lock()
            .await
            .deallocate_volume(request)
            .await?
            .into_inner();
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct VolumeGroup {
    pub(crate) data: csi::Volume,
    pub(crate) devices: Vec<Volume>,
    pub(crate) options: pond::VolumeOptions,
}

impl VolumeGroup {
    pub(crate) async fn allocate(self, secrets: &VolumeSecrets) -> Result<PersistentVolume> {
        let Self {
            data,
            devices,
            options,
        } = self;

        // Allocate volumes
        let devices = devices
            .into_iter()
            .map(|device| device.allocate(secrets.clone(), options.clone()))
            .collect::<FuturesOrdered<_>>()
            .try_collect()
            .await?;

        Ok(PersistentVolume {
            condition: csi::VolumeCondition {
                abnormal: false,
                message: "Provisioned".into(),
            },
            group: Self {
                data,
                devices,
                options,
            },
            published_node_ids: Default::default(),
            readonly: false,
        })
    }

    pub(crate) async fn deallocate(&self, secrets: &VolumeSecrets) -> Result<()> {
        let Self {
            data: _,
            devices,
            options,
        } = self;

        // Deallocate volumes
        devices
            .into_iter()
            .map(|device| device.deallocate(secrets.clone(), options.clone()))
            .collect::<FuturesOrdered<_>>()
            .try_collect::<()>()
            .await?;

        Ok(())
    }
}

pub(crate) trait VolumeOptionsExt {
    fn apply_volume_capability(&mut self, volume_capability: csi::VolumeCapability) -> Result<()>;

    fn apply_volume_capabilities(
        &mut self,
        volume_capabilities: Vec<csi::VolumeCapability>,
    ) -> Result<()> {
        volume_capabilities
            .into_iter()
            .map(|volume_capability| self.apply_volume_capability(volume_capability))
            .collect()
    }
}

impl VolumeOptionsExt for pond::VolumeOptions {
    fn apply_volume_capability(&mut self, volume_capability: csi::VolumeCapability) -> Result<()> {
        let csi::VolumeCapability {
            access_mode,
            access_type,
        } = volume_capability;

        // Validate access mode
        match access_mode.map(|field| field.mode()) {
            Some(
                csi::volume_capability::access_mode::Mode::MultiNodeMultiWriter
                | csi::volume_capability::access_mode::Mode::MultiNodeReaderOnly
                | csi::volume_capability::access_mode::Mode::MultiNodeSingleWriter,
            ) => self.mount_shared = true,
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
                if self.mount_shared {
                    return Err(Status::invalid_argument(
                        "Multi node PVC is not supported as a block device access mode",
                    ));
                }
                self.fs_type = None;
                self.mount_flags.clear();
                self.mount_group.clear();
            }
            Some(csi::volume_capability::AccessType::Mount(
                csi::volume_capability::MountVolume {
                    fs_type,
                    mount_flags,
                    volume_mount_group,
                },
            )) => {
                // Validate filesystem type
                if !fs_type.is_empty() {
                    self.fs_type = Some(fs_type.parse().map_err(|_| {
                        Status::invalid_argument(format!("Unsupported filesystem type: {fs_type}"))
                    })?);
                };
                self.mount_flags = mount_flags;
                self.mount_group = volume_mount_group;
            }
            None => (),
        }
        Ok(())
    }
}

pub(crate) trait VolumeParametersSource<T> {
    fn parse(self) -> Result<T>;
}

fn deserialize_into<T>(kind: &str, dict: HashMap<String, String>) -> Result<T>
where
    T: DeserializeOwned,
{
    let mut map = Map::default();
    map.extend(
        dict.into_iter()
            .map(|(key, value)| (key, Value::String(value))),
    );
    ::serde_json::from_value(Value::Object(map))
        // conceal secrets
        .map_err(|_| Status::invalid_argument(format!("Invalid {kind}")))
}

impl VolumeParametersSource<VolumeAttributes> for HashMap<String, String> {
    fn parse(self) -> Result<VolumeAttributes> {
        deserialize_into("volume attributes", self)
    }
}

impl VolumeParametersSource<VolumeAttributes> for Vec<HashMap<String, String>> {
    fn parse(self) -> Result<VolumeAttributes> {
        let mut map = Map::default();
        {
            let mut extend = |params: HashMap<String, String>| {
                map.extend(
                    params
                        .into_iter()
                        .map(|(key, value)| (key, Value::String(value))),
                )
            };
            for params in self {
                extend(params);
            }
        }
        ::serde_json::from_value(Value::Object(map))
            .map_err(|error| Status::invalid_argument(format!("Invalid attributes: {error}")))
    }
}

impl VolumeParametersSource<VolumeSecrets> for HashMap<String, String> {
    fn parse(self) -> Result<VolumeSecrets> {
        deserialize_into("volume secrets", self)
    }
}

impl VolumeParametersSource<VolumePublishControllerContext> for HashMap<String, String> {
    fn parse(self) -> Result<VolumePublishControllerContext> {
        Ok(VolumePublishControllerContext {
            devices: self
                .get(VolumePublishControllerContext::LABEL_DEVICES)
                .ok_or_else(|| Status::not_found("Empty devices"))
                .and_then(|s| {
                    ::serde_json::from_str(s).map_err(|error| {
                        Status::invalid_argument(format!("Invalid devices: {error}"))
                    })
                })?,
        })
    }
}

pub(crate) trait VolumeParametersExport {
    fn to_dict(&self) -> Result<HashMap<String, String>>;
}

fn serialize_from<T>(kind: &str, dict: T) -> Result<HashMap<String, String>>
where
    T: Serialize,
{
    ::serde_json::to_value(dict)
        .and_then(::serde_json::from_value)
        .map_err(|error| Status::internal(format!("Failed to export {kind}: {error}")))
}

impl VolumeParametersExport for VolumeAttributes {
    fn to_dict(&self) -> Result<HashMap<String, String>> {
        serialize_from("volume attributes", self)
    }
}

impl VolumeParametersExport for VolumeSecrets {
    fn to_dict(&self) -> Result<HashMap<String, String>> {
        serialize_from("volume secrets", self)
    }
}

impl VolumeParametersExport for VolumePublishControllerContext {
    fn to_dict(&self) -> Result<HashMap<String, String>> {
        let mut map = HashMap::default();
        map.insert(
            Self::LABEL_DEVICES.into(),
            ::serde_json::to_string(&self.devices).map_err(|error| {
                Status::invalid_argument(format!("Failed to export devices: {error}"))
            })?,
        );
        Ok(map)
    }
}

#[derive(Debug)]
pub(crate) struct PersistentVolume {
    pub(crate) condition: csi::VolumeCondition,
    pub(crate) group: VolumeGroup,
    pub(crate) published_node_ids: BTreeSet<String>,
    pub(crate) readonly: bool,
}
