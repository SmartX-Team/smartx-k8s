mod r#box;
mod job;

use anyhow::Result;
use clap::Parser;
use kube::Client;
use openark_core::operator::{OperatorArgs, install_crd};
use openark_kiss_api::r#box::BoxCrd;
use tokio::try_join;

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, env = "CONTROLLER_POD_NAME")]
    controller_pod_name: Option<String>,

    #[arg(long, env = "ENABLE_CRONJOBS")]
    enable_cronjobs: bool,

    #[arg(long, env = "INSTALL_CRDS")]
    install_crds: bool,

    #[arg(long, env = "NAMESPACE", value_name = "NAME")]
    namespace: String,

    #[command(flatten)]
    operator: OperatorArgs,
}

async fn install_crds(args: &OperatorArgs, client: &Client) -> Result<()> {
    install_crd::<BoxCrd>(args, client).await?;
    Ok(())
}

async fn try_main(args: Args) -> Result<()> {
    let client = Client::try_default().await?;

    // Update CRDs
    if args.install_crds {
        install_crds(&args.operator, &client).await?;
    }

    let ((), ()) = try_join!(
        {
            let args = args.clone();
            let client = client.clone();
            self::r#box::loop_forever(args, client)
        },
        self::job::loop_forever(args, client)
    )?;
    Ok(())
}

#[::tokio::main]
async fn main() {
    let args = Args::parse();

    ::openark_core::init_once();

    #[cfg(feature = "tracing")]
    ::tracing::info!("Welcome to OpenARK KISS Operator!");

    try_main(args).await.expect("running an operator")
}
