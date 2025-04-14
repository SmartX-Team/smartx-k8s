pub mod error;

#[cfg(feature = "clap")]
use clap::Parser;
use futures::{
    StreamExt,
    stream::{FuturesOrdered, FuturesUnordered},
};
use k8s_openapi::api::core::v1::Pod;
use kube::{
    Api, Client, ResourceExt,
    api::{AttachParams, AttachedProcess, ListParams},
};
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use self::error::Result;

#[cfg_attr(feature = "clap", derive(Parser))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ExecArgs {
    /// Command to be executed
    #[arg(last = true)]
    pub command: Vec<String>,

    /// Target session pod label selector
    #[arg(long)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub label_selector: Option<String>,

    /// Target session namespace
    #[arg(short = 'n', long, default_value = "vine-session")]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub namespace: Option<String>,

    /// Whether to wait the attached processes.
    #[arg(short, long)]
    #[cfg_attr(feature = "serde", serde(default))]
    pub wait: bool,
}

pub async fn exec(kube: Client, args: &ExecArgs) -> Result<ExecSession> {
    let ExecArgs {
        command,
        label_selector,
        namespace,
        wait,
    } = args;

    // List session pods
    let api: Api<Pod> = match namespace.as_deref() {
        Some(ns) => Api::namespaced(kube, ns),
        None => Api::default_namespaced(kube),
    };
    let lp = ListParams {
        label_selector: label_selector.clone(),
        ..Default::default()
    };
    let pods = api.list(&lp).await?.items;

    // Create processes
    let container = "session";
    let wait = *wait;
    let ap = AttachParams {
        container: Some(container.into()),
        stdin: false,
        stdout: true,
        stderr: !wait,
        tty: wait,
        max_stdin_buf_size: None,
        max_stdout_buf_size: None,
        max_stderr_buf_size: None,
    };
    let processes: Vec<_> = pods
        .iter()
        .filter(|&pod| {
            // Check the session is ready
            pod.status
                .as_ref()
                .and_then(|status| status.container_statuses.as_ref())
                .and_then(|statuses| statuses.iter().find(|&status| status.name == container))
                .is_some_and(|status| status.ready)
        })
        .map(|pod| async {
            let name = pod.name_any();
            match api.exec(&name, command, &ap).await {
                Ok(attached) => Some(Process { attached, name }),
                Err(error) => {
                    #[cfg(feature = "tracing")]
                    {
                        ::tracing::error!("Failed to exec to {name}: {error}");
                    }
                    None
                }
            }
        })
        .collect::<FuturesOrdered<_>>()
        .collect()
        .await;

    // Collect processes
    Ok(ExecSession {
        processes: processes.into_iter().flatten().collect(),
    })
}

struct Process {
    attached: AttachedProcess,
    name: String,
}

impl Process {
    async fn join(self) {
        let Self { attached, name } = self;

        match attached.join().await {
            Ok(()) => (),
            Err(error) => {
                #[cfg(feature = "tracing")]
                {
                    ::tracing::error!("Failed to exec to {name}: {error}");
                }
            }
        }
    }
}

pub struct ExecSession {
    processes: Vec<Process>,
}

impl ExecSession {
    pub async fn join(self) {
        // Spawn processes
        let processes: Vec<_> = self
            .processes
            .into_iter()
            .map(|process| ::tokio::spawn(process.join()))
            .collect();

        // Join processes
        processes
            .into_iter()
            .map(|process| async move {
                match process.await {
                    Ok(()) => (),
                    Err(error) => {
                        #[cfg(feature = "tracing")]
                        {
                            ::tracing::error!("Failed to join: {error}");
                        }
                    }
                }
            })
            .collect::<FuturesUnordered<_>>()
            .collect()
            .await
    }
}
