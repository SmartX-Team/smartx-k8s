use std::{collections::BTreeMap, path::PathBuf, time::Duration};

use anyhow::{Context, Result, bail, ensure};
use clap::Parser;
use k8s_openapi::{api::core::v1::ConfigMap, apimachinery::pkg::apis::meta::v1::OwnerReference};
use kube::{
    Api, Client,
    api::{ObjectMeta, Patch, PatchParams},
};
use openark_rook_ceph_controller::{device, model::inventory_name};
use tokio::time::{MissedTickBehavior, interval};

const FIELD_MANAGER: &str = "openark-rook-ceph-agent";

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, env = "POD_NAMESPACE")]
    namespace: String,

    #[arg(long, env = "POD_NAME")]
    pod_name: String,

    #[arg(long, env = "POD_UID")]
    pod_uid: String,

    #[arg(long, env = "APP_INSTANCE")]
    app_instance: String,

    #[arg(long, env = "INVENTORY_INTERVAL_SECONDS", default_value_t = 300)]
    interval_seconds: u64,

    #[arg(long, env = "HOST_DEV_ROOT", default_value = "/host/dev")]
    host_dev_root: PathBuf,

    #[arg(long, env = "HOST_SYS_ROOT", default_value = "/host/sys")]
    host_sys_root: PathBuf,
}

async fn publish(args: &Args, config_maps: &Api<ConfigMap>, name: &str) -> Result<()> {
    let inventory = device::scan(&args.host_dev_root, &args.host_sys_root)?;
    let device_count = inventory.devices.len();
    let inventory = serde_json::to_string(&inventory).context("serializing device inventory")?;
    let labels = BTreeMap::from([
        (
            "app.kubernetes.io/instance".to_owned(),
            args.app_instance.clone(),
        ),
        (
            "app.kubernetes.io/component".to_owned(),
            "inventory".to_owned(),
        ),
        (
            "org.ulagbulag.io/rook-ceph-device-inventory".to_owned(),
            "true".to_owned(),
        ),
    ]);
    let owner_references = vec![OwnerReference {
        api_version: "v1".to_owned(),
        block_owner_deletion: Some(false),
        controller: Some(false),
        kind: "Pod".to_owned(),
        name: args.pod_name.clone(),
        uid: args.pod_uid.clone(),
    }];
    let config_map = ConfigMap {
        metadata: ObjectMeta {
            name: Some(name.to_owned()),
            namespace: Some(args.namespace.clone()),
            labels: Some(labels),
            owner_references: Some(owner_references),
            ..Default::default()
        },
        data: Some(BTreeMap::from([("inventory.json".to_owned(), inventory)])),
        ..Default::default()
    };
    let params = PatchParams {
        dry_run: false,
        force: true,
        field_manager: Some(FIELD_MANAGER.to_owned()),
        field_validation: None,
    };
    config_maps
        .patch(name, &params, &Patch::Apply(&config_map))
        .await
        .with_context(|| format!("applying inventory ConfigMap {}/{}", args.namespace, name))?;
    #[cfg(feature = "tracing")]
    tracing::info!(
        config_map = name,
        devices = device_count,
        "published device inventory"
    );
    #[cfg(not(feature = "tracing"))]
    let _ = device_count;
    Ok(())
}

async fn try_main(args: Args) -> Result<()> {
    ensure!(
        !args.namespace.is_empty(),
        "Pod namespace must not be empty"
    );
    ensure!(!args.pod_name.is_empty(), "Pod name must not be empty");
    ensure!(
        !args.app_instance.is_empty(),
        "app instance must not be empty"
    );
    if args.interval_seconds == 0 {
        bail!("inventory interval must be greater than zero");
    }
    let name = inventory_name(&args.pod_uid)?;

    let client = Client::try_default()
        .await
        .context("creating Kubernetes client")?;
    let config_maps = Api::<ConfigMap>::namespaced(client, &args.namespace);
    let mut ticker = interval(Duration::from_secs(args.interval_seconds));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        ticker.tick().await;
        publish(&args, &config_maps, &name).await?;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    openark_core::init_once();
    #[cfg(feature = "tracing")]
    tracing::info!("Welcome to OpenARK Rook Ceph Inventory Agent!");
    try_main(args).await
}
