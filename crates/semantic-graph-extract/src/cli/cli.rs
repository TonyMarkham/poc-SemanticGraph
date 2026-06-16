use crate::cli::Command;

use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(about = "Language-server-backed semantic graph extraction prototype")]
pub struct Cli {
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}
