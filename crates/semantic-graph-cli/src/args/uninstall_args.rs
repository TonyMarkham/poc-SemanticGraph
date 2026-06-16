use crate::args::UninstallCommand;
use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct UninstallArgs {
    #[command(subcommand)]
    pub command: UninstallCommand,
}
