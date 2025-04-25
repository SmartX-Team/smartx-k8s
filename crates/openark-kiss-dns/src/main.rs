mod reloader;

use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::{Result, bail};
use async_trait::async_trait;
use clap::Parser;
use hickory_server::{
    ServerFuture,
    authority::Catalog,
    server::{Request, RequestHandler, ResponseHandler, ResponseInfo},
};
use kube::Client;
use tokio::{
    net::{TcpListener, UdpSocket},
    spawn,
    sync::RwLock,
};
#[cfg(feature = "tracing")]
use tracing::info;

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// An address to bind the server
    #[arg(
        long,
        env = "BIND_ADDR",
        value_name = "ADDR",
        default_value = "0.0.0.0:53"
    )]
    bind_addr: SocketAddr,

    /// Kubernetes cluster domain name
    #[arg(
        long,
        env = "CLUSTER_DOMAIN_NAME",
        value_name = "NAME",
        default_value = "cluster.local"
    )]
    cluster_domain_name: String,
}

#[derive(Clone)]
struct Handler {
    catalog: Arc<RwLock<Catalog>>,
}

impl Handler {
    async fn try_default() -> Result<Self> {
        Ok(Self {
            catalog: Arc::default(),
        })
    }
}

#[async_trait]
impl RequestHandler for Handler {
    async fn handle_request<R>(&self, request: &Request, response_handle: R) -> ResponseInfo
    where
        R: ResponseHandler,
    {
        self.catalog
            .read()
            .await
            .handle_request(request, response_handle)
            .await
        // let mut header = Header::new();
        // header.set_message_type(MessageType::Response);
        // header.into()
    }
}

async fn build_server(addr: SocketAddr, handler: Handler) -> Result<ServerFuture<Handler>> {
    let mut server = ServerFuture::new(handler);

    let socket = UdpSocket::bind(addr).await?;
    server.register_socket(socket);

    let listener = TcpListener::bind(addr).await?;
    let timeout = Duration::from_secs(30);
    server.register_listener(listener, timeout);

    Ok(server)
}

async fn try_main(args: &Args) -> Result<()> {
    #[cfg(feature = "tracing")]
    info!("Booting...");

    let handler = match Handler::try_default().await {
        Ok(handler) => handler,
        Err(error) => bail!("failed to init handler: {error}"),
    };
    let mut server = match build_server(args.bind_addr, handler.clone()).await {
        Ok(server) => server,
        Err(error) => bail!("failed to init server: {error}"),
    };

    let kube = match Client::try_default().await {
        Ok(kube) => kube,
        Err(error) => bail!("failed to init kubernetes client: {error}"),
    };

    let ctx = match self::reloader::ReloaderContext::try_new(&args.cluster_domain_name).await {
        Ok(ctx) => ctx,
        Err(error) => bail!("failed to init reloader context: {error}"),
    };

    #[cfg(feature = "tracing")]
    info!("Registering side workers...");
    let workers = vec![spawn(self::reloader::loop_forever(ctx, kube, handler))];

    #[cfg(feature = "tracing")]
    info!("Ready");
    let result = server.block_until_done().await;

    #[cfg(feature = "tracing")]
    info!("Terminating...");
    for worker in workers {
        worker.abort();
    }
    result.map_err(Into::into)
}

#[::tokio::main]
async fn main() {
    let args = Args::parse();

    ::openark_core::init_once();

    #[cfg(feature = "tracing")]
    ::tracing::info!("Welcome to OpenARK KISS DNS!");

    loop {
        try_main(&args).await.expect("running a server")
    }
}
