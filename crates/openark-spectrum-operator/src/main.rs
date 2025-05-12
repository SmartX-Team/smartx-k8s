mod histogram;
mod metrics_class;
mod pool;
mod pool_claim;
mod status;
mod targets;
mod utils;

use anyhow::Result;
use clap::Parser;
use kube::Client;
use openark_core::operator::{OperatorArgs, install_crd};
use openark_spectrum_api::{
    histogram::HistogramCrd, metrics_class::MetricsClassCrd, pool::PoolCrd,
    pool_claim::PoolClaimCrd,
};
use tokio::try_join;
use url::Url;

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, env = "OPENARK_LABEL_SPECTRUM_HISTOGRAM_PARENT")]
    label_histogram_parent: String,

    #[arg(long, env = "OPENARK_LABEL_SPECTRUM_HISTOGRAM_WEIGHT")]
    label_histogram_weight: String,

    #[arg(long, env = "OPENARK_LABEL_SPECTRUM_POOL_CLAIM_LIFECYCLE_PRE_START")]
    label_pool_claim_lifecycle_pre_start: String,

    #[arg(long, env = "OPENARK_LABEL_SPECTRUM_POOL_CLAIM_PARENT")]
    label_pool_claim_parent: String,

    #[arg(long, env = "OPENARK_LABEL_SPECTRUM_POOL_CLAIM_PRIORITY")]
    label_pool_claim_priority: String,

    #[arg(long, env = "OPENARK_LABEL_SPECTRUM_POOL_CLAIM_WEIGHT")]
    label_pool_claim_weight: String,

    #[arg(long, env = "OPENARK_LABEL_SPECTRUM_POOL_CLAIM_WEIGHT_PENALTY")]
    label_pool_claim_weight_penalty: String,

    #[arg(long, env = "OPENARK_LABEL_SPECTRUM_POOL_CLAIM_WEIGHT_MAX")]
    label_pool_claim_weight_max: String,

    #[arg(long, env = "OPENARK_LABEL_SPECTRUM_POOL_CLAIM_WEIGHT_MIN")]
    label_pool_claim_weight_min: String,

    #[arg(long, env = "OPENARK_LABEL_SPECTRUM_POOL_PARENT")]
    label_pool_parent: String,

    #[command(flatten)]
    operator: OperatorArgs,

    #[arg(long, env = "OPENARK_SPECTRUM_POOL_BASE_URL")]
    pool_base_url: Url,
}

async fn install_crds(args: &OperatorArgs, client: &Client) -> Result<()> {
    install_crd::<HistogramCrd>(args, client).await?;
    install_crd::<MetricsClassCrd>(args, client).await?;
    install_crd::<PoolClaimCrd>(args, client).await?;
    install_crd::<PoolCrd>(args, client).await?;
    Ok(())
}

async fn try_main(args: Args) -> Result<()> {
    let client = ::reqwest::Client::new();
    let kube = Client::try_default().await?;

    // Update CRDs
    if args.operator.install_crds {
        install_crds(&args.operator, &kube).await?;
    }

    let ((), (), (), ()) = try_join!(
        {
            let args = args.clone();
            let client = client.clone();
            let kube = kube.clone();
            self::histogram::loop_forever(args, client, kube)
        },
        {
            let args = args.clone();
            let client = client.clone();
            let kube = kube.clone();
            self::metrics_class::loop_forever(args, client, kube)
        },
        {
            let args = args.clone();
            let kube = kube.clone();
            self::pool::loop_forever(args, client, kube)
        },
        self::pool_claim::loop_forever(args, kube),
    )?;
    Ok(())
}

#[::tokio::main]
async fn main() {
    let args = Args::parse();

    ::openark_core::init_once();

    #[cfg(feature = "tracing")]
    ::tracing::info!("Welcome to OpenARK Spectrum Operator!");

    try_main(args).await.expect("running an operator")
}
