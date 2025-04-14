pub mod error;

use std::{borrow::Cow, time::Duration};

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
use tokio::time::sleep;

use self::error::Result;

#[cfg_attr(feature = "clap", derive(Parser))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ExecArgs {
    /// Command to be executed
    #[cfg_attr(feature = "clap", arg(last = true))]
    pub command: Vec<String>,

    /// Target session pod label selector
    #[cfg_attr(feature = "clap", arg(long))]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub label_selector: Option<String>,

    /// Target session namespace
    #[cfg_attr(
        feature = "clap",
        arg(short = 'n', long, default_value = "vine-session")
    )]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub namespace: Option<String>,

    /// Whether to excute within a GUI terminal.
    #[cfg_attr(feature = "clap", arg(short, long))]
    #[cfg_attr(feature = "serde", serde(default))]
    pub terminal: bool,

    /// Whether to wait the attached processes.
    #[cfg_attr(feature = "clap", arg(short, long))]
    #[cfg_attr(feature = "serde", serde(default))]
    pub wait: bool,
}

pub async fn exec(kube: Client, args: &ExecArgs) -> Result<ExecSession> {
    let ExecArgs {
        command,
        label_selector,
        namespace,
        terminal,
        wait,
    } = args;

    // Convert commands
    let command = if *terminal {
        let mut wrapped_command = vec!["xfce4-terminal".into(), "--execute".into()];
        wrapped_command.extend_from_slice(command);
        Cow::Owned(wrapped_command)
    } else {
        Cow::Borrowed(command)
    };

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
            match api.exec(&name, command.as_slice(), &ap).await {
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
        wait,
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
    wait: bool,
}

impl ExecSession {
    pub async fn join(self) {
        let Self { processes, wait } = self;

        // Spawn processes
        let processes: Vec<_> = processes
            .into_iter()
            .map(|process| {
                let name = process.name.clone();
                #[cfg(feature = "tracing")]
                {
                    ::tracing::debug!("Executed: {name}");
                }
                (name, ::tokio::spawn(process.join()))
            })
            .collect();

        let num_processes = processes.len();

        // Join processes
        if wait {
            processes
                .into_iter()
                .map(|(name, process)| async move {
                    match process.await {
                        Ok(()) => {
                            #[cfg(feature = "tracing")]
                            {
                                ::tracing::info!("Completed: {name}");
                            }
                            let _ = name;
                        }
                        Err(error) => {
                            #[cfg(feature = "tracing")]
                            {
                                ::tracing::error!("Failed to join: {error}");
                            }
                        }
                    }
                })
                .collect::<FuturesUnordered<_>>()
                .collect::<()>()
                .await;

            #[cfg(feature = "tracing")]
            {
                ::tracing::info!("Completed at {num_processes} sessions");
            }
        } else {
            sleep(Duration::from_secs(1)).await;

            #[cfg(feature = "tracing")]
            {
                ::tracing::info!("Spawned {num_processes} sessions");
            }
        }
    }
}
