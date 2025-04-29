mod endpoint_slice;
mod http_route_claim;
mod traffic_route_claim;
mod traffic_router_class;

use anyhow::Result;
use clap::Parser;
use kube::Client;
use openark_core::operator::{OperatorArgs, install_crd};
use openark_traffic_api::{
    http_route_claim::HTTPRouteClaimCrd, traffic_router_class::TrafficRouterClassCrd,
};
use tokio::try_join;

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(flatten)]
    operator: OperatorArgs,
}

async fn install_crds(args: &OperatorArgs, client: &Client) -> Result<()> {
    install_crd::<HTTPRouteClaimCrd>(args, client).await?;
    install_crd::<TrafficRouterClassCrd>(args, client).await?;
    Ok(())
}

async fn try_main(args: Args) -> Result<()> {
    let client = Client::try_default().await?;

    // Update CRDs
    if args.operator.install_crds {
        install_crds(&args.operator, &client).await?;
    }

    let ((), ()) = try_join!(
        // {
        //     let args = args.clone();
        //     let client = client.clone();
        //     self::endpoint_slice::loop_forever(args, client)
        // },
        {
            let args = args.clone();
            let client = client.clone();
            self::http_route_claim::loop_forever(args, client)
        },
        self::traffic_router_class::loop_forever(args, client)
    )?;
    Ok(())
}

#[::tokio::main]
async fn main() {
    let args = Args::parse();

    ::openark_core::init_once();

    #[cfg(feature = "tracing")]
    ::tracing::info!("Welcome to OpenARK Gateway Route Operator!");

    try_main(args).await.expect("running an operator")
}
