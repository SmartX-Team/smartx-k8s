use anyhow::Result;
use clap::Parser;
use kube::Client;
use openark_vine_session_api::exec::ExecArgs;

/// Execute a command into multiple vine sessions.
#[derive(Parser)]
pub(crate) struct Args {
    #[command(flatten)]
    exec: ExecArgs,
}

impl Args {
    pub(super) async fn exec(self) -> Result<()> {
        let Self { exec: args } = self;

        let kube = Client::try_default().await?;

        let session = ::openark_vine_session_exec::exec(kube, &args).await?;
        session.join().await;
        Ok(())
    }
}
