use std::{path::PathBuf, process::exit};

use clap::Parser;
use connected_data_lake_api::{error::Result, fs::FilesystemExt};
use fuser::MountOption;

const FILESYSTEM_NAME: &str = "cdlfs";

#[derive(Debug, Parser)]
struct Args {
    directory: PathBuf,
}

fn main() {
    let args = Args::parse();

    ::openark_core::init_once();

    #[cfg(feature = "tracing")]
    ::tracing::debug!("Welcome to Connected Data Lake Driver!");

    match try_main(args) {
        Ok(()) => (),
        Err(error) => {
            #[cfg(feature = "tracing")]
            {
                ::tracing::error!("{error}");
                exit(1)
            }
        }
    }
}

fn try_main(args: Args) -> Result<()> {
    let Args {
        directory: mountpoint,
    } = args;

    let filesystem = ::connected_data_lake_layer_staging::fs::Filesystem::<
        ::connected_data_lake_layer_staging::block::BlockDevice,
    >::default()
    .into_fuse();

    ::fuser::mount2(
        filesystem,
        mountpoint,
        &[
            MountOption::AllowRoot,
            MountOption::Async,
            MountOption::Atime,
            MountOption::AutoUnmount,
            MountOption::Exec,
            MountOption::FSName(FILESYSTEM_NAME.into()),
            MountOption::NoDev,
            MountOption::RW,
            MountOption::Suid,
        ],
    )?;
    Ok(())
}
