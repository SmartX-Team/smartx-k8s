mod batch;

use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum Args {
    Batch(self::batch::Args),
}

impl Args {
    pub(super) async fn exec(self) -> Result<()> {
        match self {
            Self::Batch(args) => args.exec().await,
        }
    }
}
