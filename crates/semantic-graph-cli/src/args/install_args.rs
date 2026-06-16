use crate::args::InstallCommand;
use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct InstallArgs {
    #[command(subcommand)]
    pub command: InstallCommand,
}
