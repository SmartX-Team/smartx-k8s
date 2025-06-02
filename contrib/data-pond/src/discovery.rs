use std::{
    collections::{BTreeSet, HashMap},
    process::Stdio,
};

use anyhow::{Result, bail};
use data_pond_csi::pond;
use futures::{TryStreamExt, stream::FuturesOrdered};
use serde::Deserialize;
use tokio::process::Command;
#[cfg(feature = "tracing")]
use tracing::warn;

#[derive(Debug, Deserialize)]
struct Device {
    id: String,
    uid: Option<String>,
    layer: pond::device_layer::Type,
    source: pond::device_source::Type,
    capacity: i64,
}

pub(crate) async fn discover_devices(
    server: &super::Server,
) -> Result<HashMap<String, pond::Device>> {
    server
        .sources
        .iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|source| async move {
            let program = format!("./{source}-source-kernel-discover.sh");
            let output = Command::new(program)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .output()
                .await
                .unwrap();
            if !output.status.success() {
                bail!(
                    "Failed to discover {source} sources: {code}",
                    code = output.status.code().unwrap_or(-1),
                )
            }

            let stdout = String::from_utf8(output.stdout)?;

            stdout
                .split('\n')
                .map(|line| line.trim())
                .filter(|&line| !line.is_empty())
                .filter_map(|line| match ::serde_json::from_str::<Device>(line) {
                    Ok(pond) => Some(Ok(pond)),
                    Err(error) => {
                        #[cfg(feature = "tracing")]
                        warn!("Failed to parse device: {error}");
                        let _ = error;
                        None
                    }
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<FuturesOrdered<_>>()
        .try_collect::<Vec<Vec<_>>>()
        .await
        .map(|lists| {
            lists
                .into_iter()
                .flatten()
                .map(
                    |Device {
                         id,
                         uid,
                         layer,
                         source,
                         capacity,
                     }| {
                        let device = pond::Device {
                            id: id.clone(),
                            pond_id: server.node_id.clone(),
                            uid,
                            layer: layer.into(),
                            source: source.into(),
                            capacity,
                        };
                        (id, device)
                    },
                )
                .collect()
        })
}

pub(crate) async fn discover_volumes(
    server: &super::Server,
) -> Result<HashMap<String, pond::VolumeBindingMetadata>> {
    server
        .layers
        .iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|layer| async move {
            let program = format!("./{layer}-discover.sh");
            let output = Command::new(program)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .output()
                .await
                .unwrap();
            if !output.status.success() {
                bail!(
                    "Failed to discover {layer} layers: {code}",
                    code = output.status.code().unwrap_or(-1),
                )
            }

            let stdout = String::from_utf8(output.stdout)?;

            stdout
                .split('\n')
                .map(|line| line.trim())
                .filter(|&line| !line.is_empty())
                .filter_map(|line| {
                    match ::serde_json::from_str::<pond::VolumeBindingMetadata>(line) {
                        Ok(pond) => Some(Ok(pond)),
                        Err(error) => {
                            #[cfg(feature = "tracing")]
                            warn!("Failed to parse volume: {error}");
                            let _ = error;
                            None
                        }
                    }
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<FuturesOrdered<_>>()
        .try_collect::<Vec<Vec<_>>>()
        .await
        .map(|lists| {
            lists
                .into_iter()
                .flatten()
                .map(|metadata| {
                    let id = metadata.volume_id.clone();
                    (id, metadata)
                })
                .collect()
        })
}
