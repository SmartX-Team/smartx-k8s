mod session;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

impl Args {
    async fn exec(self) {
        match self.command.exec().await {
            Ok(()) => (),
            Err(error) => {
                #[cfg(feature = "tracing")]
                {
                    ::tracing::error!("{error}");
                }
            }
        }
    }
}

#[derive(Subcommand)]
enum Command {
    #[command(flatten)]
    Session(self::session::Args),
}

impl Command {
    async fn exec(self) -> Result<()> {
        match self {
            Self::Session(args) => args.exec().await,
        }
    }
}

#[::tokio::main]
async fn main() {
    let args = Args::parse();

    ::openark_core::init_once();

    #[cfg(feature = "tracing")]
    ::tracing::debug!("Welcome to OpenARK!");

    args.exec().await
}
