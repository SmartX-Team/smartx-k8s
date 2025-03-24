use std::process::{Stdio, exit};

use anyhow::{Result, bail};
use clap::Parser;
use futures::TryStreamExt;
use k8s_openapi::{Resource, api::core::v1::Node};
use kube::{
    Api, Client,
    api::{PartialObjectMeta, Patch, PatchParams},
    runtime::{
        metadata_watcher,
        watcher::{Config, Event},
    },
};
use openark_vine_session_api::filter_taint;
use serde_json::json;
use tokio::process::Command;
#[cfg(feature = "tracing")]
use tracing::{Level, info, instrument, warn};

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, env = "CONTROLLER_NAME")]
    controller_name: String,

    #[arg(long, env = "DRY_RUN")]
    dry_run: bool,

    #[arg(long, env = "OPENARK_LABEL_SIGNED_OUT")]
    label_signed_out: String,

    #[arg(long, env = "NODE_NAME")]
    node_name: String,

    #[arg(long, env = "SERVICE_NAME")]
    service_name: String,
}

struct Service<'a> {
    api: Api<Node>,
    args: &'a Args,
    patch_params: PatchParams,
    running: Option<bool>,
}

impl Service<'_> {
    #[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
    async fn apply(&mut self, node: &PartialObjectMeta<Node>) -> Result<()> {
        let signed_out = node
            .metadata
            .labels
            .as_ref()
            .and_then(|map| map.get(&self.args.label_signed_out))
            .and_then(|value| value.parse().ok())
            .unwrap_or(true);

        if signed_out {
            self.start().await
        } else {
            self.stop().await
        }
    }

    #[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
    async fn start(&mut self) -> Result<()> {
        const NEXT: Option<bool> = Some(true);
        if self.running == NEXT {
            return Ok(());
        }

        #[cfg(feature = "tracing")]
        info!("Starting service");

        if !self.args.dry_run {
            systemctl_exec("start", &self.args.service_name).await?;
        } else {
            #[cfg(feature = "tracing")]
            info!("dry_run: systemctl start {}", &self.args.service_name)
        }

        #[cfg(feature = "tracing")]
        info!("Started service");

        self.running = NEXT;
        Ok(())
    }

    #[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
    async fn stop(&mut self) -> Result<()> {
        const NEXT: Option<bool> = Some(false);
        if self.running == NEXT {
            return Ok(());
        }

        #[cfg(feature = "tracing")]
        info!("Stopping service");

        if !self.args.dry_run {
            systemctl_exec("stop", &self.args.service_name).await?
        } else {
            #[cfg(feature = "tracing")]
            info!("dry_run: systemctl stop {}", &self.args.service_name)
        }

        #[cfg(feature = "tracing")]
        info!("Stopped service");

        self.untaint().await?;
        self.running = NEXT;
        Ok(())
    }

    #[cfg_attr(feature = "tracing", instrument(level = Level::INFO, skip_all))]
    async fn untaint(&self) -> Result<()> {
        let name = &self.args.node_name;
        let node = self.api.get(name).await?;

        // Remove taint
        let last_taints = node.spec.as_ref().and_then(|spec| spec.taints.as_ref());
        let next_taints: Vec<_> = last_taints
            .map(|taints| {
                taints
                    .iter()
                    .filter(|&taint| !filter_taint(taint, &self.args.label_signed_out))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        // Apply the updated taints
        let patch = Patch::Strategic(json!({
            "apiVersion": Node::API_VERSION,
            "kind": Node::KIND,
            "metadata": {
                "name": name,
            },
            "spec": {
                "taints": next_taints,
                "unschedulable": false,
            },
        }));
        self.api.patch(name, &self.patch_params, &patch).await?;
        {
            #[cfg(feature = "tracing")]
            info!("untainted node/{name}");
        }
        Ok(())
    }
}

#[cfg_attr(feature = "tracing", instrument(level = Level::INFO))]
async fn systemctl_exec(command: &str, service_name: &str) -> Result<()> {
    let status = Command::new("systemctl")
        .args([command, service_name])
        .kill_on_drop(true)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await?;

    match status.code() {
        Some(0) | None => Ok(()),
        Some(code) => exit(code),
    }
}

async fn try_main(args: Args) -> Result<()> {
    // Enable the service for the next boot
    systemctl_exec("enable", &args.service_name).await?;

    let client = Client::try_default().await?;
    let api = Api::all(client.clone());

    let mut service = Service {
        api: api.clone(),
        args: &args,
        patch_params: PatchParams {
            dry_run: args.dry_run,
            force: false,
            field_manager: Some(args.controller_name.clone()),
            field_validation: None,
        },
        running: None,
    };

    let watcher_config = Config {
        label_selector: Some(format!("kubernetes.io/hostname={}", &args.node_name)),
        ..Default::default()
    };

    let mut stream = Box::pin(metadata_watcher(api, watcher_config));
    while let Some(event) = stream.try_next().await? {
        match event {
            Event::Apply(node) | Event::InitApply(node) => service.apply(&node).await?,
            Event::Delete(_) => {
                {
                    #[cfg(feature = "tracing")]
                    warn!("deleting node: {}", &args.node_name);
                }
                service.stop().await?
            }
            Event::Init | Event::InitDone => continue,
        }
    }
    bail!("unexpected termination")
}

#[::tokio::main]
async fn main() {
    let args = Args::parse();

    ::openark_core::init_once();

    #[cfg(feature = "tracing")]
    ::tracing::info!("Welcome to OpenARK VINE Session Handler!");

    try_main(args).await.expect("running a handler")
}
