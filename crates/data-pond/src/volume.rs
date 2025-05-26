use std::{collections::HashMap, process::Stdio, sync::Arc};

use async_trait::async_trait;
use data_pond_api::{
    VolumeAllocateContext, VolumeAttributes, VolumePublishControllerContext, VolumeSecrets,
};
use data_pond_csi::{
    csi,
    pond::{self, pond_client::PondClient},
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Map, Value};
use tokio::{io::AsyncWriteExt, process::Command, sync::Mutex};
use tonic::{Result, Status, transport::Channel};

#[derive(Debug)]
pub(crate) struct Pond {
    pub(crate) bindings: Vec<pond::VolumeBindingMetadata>,
    pub(crate) client: Mutex<PondClient<Channel>>,
    pub(crate) devices: Vec<pond::Device>,
    pub(crate) id: String,
    pub(crate) topology: pond::DeviceTopology,
}

#[async_trait]
pub(crate) trait PondVolumeAllocate {
    async fn execute(&self, kind: &str) -> Result<()>;

    #[inline]
    async fn allocate(&self) -> Result<()> {
        self.execute("allocate").await
    }
}

#[async_trait]
impl PondVolumeAllocate for VolumeAllocateContext<'_> {
    async fn execute(&self, kind: &str) -> Result<()> {
        let layer = self.parameters.attributes.layer;
        let program = format!("./{layer}-{kind}.sh");
        let mut process = Command::new(program)
            .stdin(Stdio::piped())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?;

        // Serialize inputs
        let inputs = ::serde_json::to_vec(self)
            .map_err(|_| Status::internal("Failed to serialize the context"))?;

        // Feed inputs
        {
            let mut stdin = process.stdin.take().unwrap();
            stdin.write_all(&inputs).await?;
            stdin.flush().await?;
        }

        // Validate the process
        let status = process.wait().await?;
        if status.success() {
            Ok(())
        } else {
            Err(Status::internal(format!(
                "Failed to {kind} {volume_id} into {device_id}: {code}",
                code = status.code().unwrap_or(-1),
                device_id = self.binding.metadata.device_id,
                volume_id = self.binding.metadata.volume_id,
            )))
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct VolumeBindingClaim {
    pub(crate) attributes: VolumeAttributes,
    pub(crate) device: pond::Device,
    pub(crate) metadata: pond::VolumeBindingMetadata,
    pub(crate) pond: Arc<Pond>,
}

impl VolumeBindingClaim {
    #[inline]
    pub(crate) fn available(&self) -> i64 {
        self.device.capacity - self.metadata.offset - self.padding()
    }

    #[inline]
    fn padding(&self) -> i64 {
        self.device.layer().padding()
    }

    pub(crate) fn reserve(&mut self, remaining: i64) -> i64 {
        let available = self.available();
        let reserved = available.min(remaining);
        self.metadata.reserved = reserved + self.padding();
        reserved
    }

    fn build(
        &self,
        secrets: VolumeSecrets,
        options: pond::VolumeOptions,
    ) -> Result<pond::AllocateVolumeRequest> {
        Ok(pond::AllocateVolumeRequest {
            attributes: self.attributes.to_dict()?,
            binding: Some(self.metadata.clone()),
            device_id: self.device.id.clone(),
            options: Some(options),
            secrets: secrets.to_dict()?,
        })
    }

    pub(crate) async fn allocate(
        &mut self,
        secrets: VolumeSecrets,
        options: pond::VolumeOptions,
    ) -> Result<()> {
        let request = self.build(secrets, options)?;
        let pond::AllocateVolumeResponse {} = self
            .pond
            .client
            .lock()
            .await
            .allocate_volume(request)
            .await?
            .into_inner();
        Ok(())
    }

    pub(crate) async fn deallocate(
        &self,
        secrets: VolumeSecrets,
        options: pond::VolumeOptions,
    ) -> Result<()> {
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
            bindings: self
                .get(VolumePublishControllerContext::LABEL_BINDINGS)
                .ok_or_else(|| Status::not_found("Empty bindings"))
                .and_then(|s| {
                    ::serde_json::from_str(s).map_err(|error| {
                        Status::invalid_argument(format!("Invalid bindings: {error}"))
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
        let Self { bindings } = self;

        let mut map = HashMap::default();
        map.insert(
            Self::LABEL_BINDINGS.into(),
            ::serde_json::to_string(bindings).map_err(|error| {
                Status::invalid_argument(format!("Failed to export bindings: {error}"))
            })?,
        );
        Ok(map)
    }
}
