use std::{path::PathBuf, process::Stdio};

use anyhow::{Error, Result};
use async_trait::async_trait;
use clap::Parser;
use kube::{
    api::DynamicObject,
    core::admission::{AdmissionRequest, AdmissionResponse},
};
use openark_admission_controller_base::AdmissionControllerBuilder;
use openark_admission_openapi::AdmissionResult;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
    try_join,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct AdmissionController {
    /// A path to the reviewer script
    #[arg(long, env = "SCRIPT_PATH", value_name = "PATH")]
    path: PathBuf,
}

#[async_trait]
impl AdmissionControllerBuilder for AdmissionController {
    type Args = Self;

    #[inline]
    async fn build(args: Self::Args) -> Result<Self> {
        Ok(args)
    }
}

#[async_trait]
impl ::openark_admission_controller_base::AdmissionController for AdmissionController {
    type Object = DynamicObject;

    async fn handle(&self, request: AdmissionRequest<Self::Object>) -> Result<AdmissionResponse> {
        let request_data = ::serde_json::to_vec(&request)?;
        let capacity = request_data.len();

        let child = Command::new(&self.path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        let mut stdin = child.stdin.unwrap();
        let task_tx = async move {
            stdin.write_all(&request_data).await?;
            stdin.flush().await.map_err(Error::from)
        };

        let mut stdout = child.stdout.unwrap();
        let task_rx = async move {
            let mut buf = Vec::with_capacity(capacity);
            stdout.read_to_end(&mut buf).await?;
            ::serde_json::from_slice(&buf).map_err(Error::from)
        };

        let response = AdmissionResponse::from(&request);
        let (_, result) = try_join!(task_tx, task_rx)?;
        Ok(match result {
            AdmissionResult::Deny { message } => response.deny(message),
            AdmissionResult::Pass => response,
            AdmissionResult::Patch { operations: patch } => response.with_patch(patch)?,
        })
    }
}

#[inline]
fn main() {
    ::openark_admission_controller_base::loop_forever::<AdmissionController>()
}
