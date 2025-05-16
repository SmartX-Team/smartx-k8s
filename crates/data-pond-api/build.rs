use std::{env, path::PathBuf};

use anyhow::Result;
use tokio::{fs::File, io::AsyncWriteExt};

#[::tokio::main]
async fn main() -> Result<()> {
    // Configure environment variables
    let out_dir: PathBuf = env::var("OUT_DIR")?.parse()?;
    let url = env::var("CSI_PROTO_URL").unwrap_or_else(|_| "https://raw.githubusercontent.com/container-storage-interface/spec/refs/heads/master/csi.proto".into());

    // Cache outputs
    println!("cargo::rerun-if-env-changed=CSI_PROTO_URL");

    // Download spec
    let path = out_dir.join("csi.proto");
    {
        let client = ::reqwest::Client::default();
        let response = client.get(url).send().await?;
        let text = response.bytes().await?;
        let mut file = File::create(&path).await?;
        file.write_all(&text).await?;
    }

    // Parse spec
    let config = ::tonic_build::configure()
        .build_client(cfg!(feature = "client"))
        .build_server(cfg!(feature = "server"));
    let protos = &[path.as_path()];
    let includes = &[out_dir.as_path()];
    config.compile_protos(protos, includes)?;

    Ok(())
}
