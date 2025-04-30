mod histogram;
mod histogram_class;
mod utils;

use anyhow::Result;
use clap::Parser;
use kube::Client;
use openark_core::operator::{OperatorArgs, install_crd};
use openark_histogram_api::{histogram::HistogramCrd, histogram_class::HistogramClassCrd};
use tokio::try_join;

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, env = "OPENARK_LABEL_HISTOGRAM_PARENT")]
    label_parent: String,

    #[command(flatten)]
    operator: OperatorArgs,
}

async fn install_crds(args: &OperatorArgs, client: &Client) -> Result<()> {
    install_crd::<HistogramCrd>(args, client).await?;
    install_crd::<HistogramClassCrd>(args, client).await?;
    Ok(())
}

async fn try_main(args: Args) -> Result<()> {
    let client = ::reqwest::Client::new();
    let kube = Client::try_default().await?;

    // Update CRDs
    if args.operator.install_crds {
        install_crds(&args.operator, &kube).await?;
    }

    let ((), ()) = try_join!(
        {
            let args = args.clone();
            let client = client.clone();
            let kube = kube.clone();
            self::histogram::loop_forever(args, client, kube)
        },
        self::histogram_class::loop_forever(args, client, kube),
    )?;
    Ok(())
}

#[::tokio::main]
async fn main() {
    let args = Args::parse();

    ::openark_core::init_once();

    #[cfg(feature = "tracing")]
    ::tracing::info!("Welcome to OpenARK Histogram Operator!");

    try_main(args).await.expect("running an operator")
}
