use std::{
    env,
    path::{Path, PathBuf},
};

use anyhow::Result;
use tokio::{fs::File, io::AsyncWriteExt};

#[::tokio::main]
async fn main() -> Result<()> {
    // Configure environment variables
    let out_dir: PathBuf = env::var("OUT_DIR")?.parse()?;
    let url = env::var("CSI_PROTO_URL").unwrap_or_else(|_| "https://raw.githubusercontent.com/container-storage-interface/spec/refs/heads/master/csi.proto".into());

    // Cache outputs
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=proto/pond.proto");
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-env-changed=CSI_PROTO_URL");

    // Download spec
    let path = out_dir.join("csi.proto");
    {
        let client = ::reqwest::Client::default();
        let response = client.get(url).send().await?;
        let text = response.bytes().await?;
        let mut file = File::create(&path).await?;
        file.write_all(&text).await?;
    }

    const DERIVE_SERDE: &str = "#[derive(::serde::Serialize, ::serde::Deserialize)]";
    const DERIVE_STRUM: &str = "#[derive(::strum::Display, ::strum::EnumString)]";

    const SERDE_RENAME: &str = "#[serde(rename_all = \"snake_case\")]";
    const STRUM_RENAME: &str = "#[strum(serialize_all = \"snake_case\")]";

    // Parse spec
    let config = ::tonic_prost_build::configure()
        .build_client(cfg!(feature = "client"))
        .build_server(cfg!(feature = "server"))
        .emit_rerun_if_changed(false)
        .type_attribute("AllocateVolumeRequest", DERIVE_SERDE)
        .type_attribute("Device", DERIVE_SERDE)
        .enum_attribute("DeviceLayer.Type", DERIVE_SERDE)
        .enum_attribute("DeviceLayer.Type", DERIVE_STRUM)
        .enum_attribute("DeviceLayer.Type", SERDE_RENAME)
        .enum_attribute("DeviceLayer.Type", STRUM_RENAME)
        .enum_attribute("DeviceSource.Type", DERIVE_SERDE)
        .enum_attribute("DeviceSource.Type", DERIVE_STRUM)
        .enum_attribute("DeviceSource.Type", SERDE_RENAME)
        .enum_attribute("DeviceSource.Type", STRUM_RENAME)
        .type_attribute("VolumeBindingMetadata", DERIVE_SERDE)
        .type_attribute("VolumeOptions", DERIVE_SERDE);
    let protos = &[Path::new("proto/pond.proto"), path.as_path()];
    let includes = &[Path::new("proto"), out_dir.as_path()];
    config.compile_protos(protos, includes)?;

    Ok(())
}
