use std::time::Duration;

use clap::Parser;
use kube::{Client, Result};
use openark_core::operator::{OperatorArgs as Args, install_crd};
use openark_vine_dashboard_api::{catalog::CatalogItemCrd, table::TableCrd};
use tokio::time::sleep;

async fn install_crds(args: &Args, client: &Client) -> Result<()> {
    install_crd::<CatalogItemCrd>(args, client).await?;
    install_crd::<TableCrd>(args, client).await?;
    Ok(())
}

async fn try_main(args: Args) -> Result<()> {
    let client = Client::try_default().await?;

    // Update CRDs
    install_crds(&args, &client).await?;

    // Do nothing
    sleep(Duration::MAX).await;
    Ok(())
}

#[::tokio::main]
async fn main() {
    let args = Args::parse();

    ::openark_core::init_once();

    #[cfg(feature = "tracing")]
    ::tracing::info!("Welcome to OpenARK VINE Dashboard Operator!");

    try_main(args).await.expect("running an operator")
}
