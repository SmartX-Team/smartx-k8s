use std::{
    fs::File,
    path::{Path, PathBuf},
};

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use dark_lake_api::{kernel::DarkLake, script::Script};

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Debug, Subcommand)]
enum Command {
    /// Start a standalone daemon
    Daemon(DaemonArgs),
    /// Execute a standalone program
    Run(RunArgs),
}

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct DaemonArgs {
    path: PathBuf,
}

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct RunArgs {
    expr: String,
}

fn load_vm(path: &Path) -> Result<Script> {
    match path.extension().and_then(|s| s.to_str()) {
        Some("json") => {
            let rdr = File::open(path)?;
            ::serde_json::from_reader(rdr).map_err(Into::into)
        }
        Some(ext) => bail!("Unknown vm file type: {ext}"),
        None => bail!("Unknown vm file type"),
    }
}

fn try_main(args: Args) -> Result<()> {
    let Args { command } = args;

    let lake: DarkLake<::dark_lake_rt_io_uring::Kernel> = Default::default();
    let vm = match command {
        Command::Daemon(DaemonArgs { path }) => load_vm(&path)?,
        Command::Run(RunArgs { expr }) => lake.parse(&expr)?,
    };
    lake.wait(vm)?;

    #[cfg(feature = "tracing")]
    ::tracing::info!("Completed.");
    Ok(())
}

fn main() {
    let args = Args::parse();

    ::openark_core::init_once();

    #[cfg(feature = "tracing")]
    ::tracing::info!("Welcome to Dark Lake!");

    try_main(args).expect("running a lake")
}
