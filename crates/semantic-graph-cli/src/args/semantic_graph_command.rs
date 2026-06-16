use crate::{args::InstallArgs, constants::cli::INSTALL_COMMAND_NAME};
use clap::Subcommand;

#[derive(Clone, Debug, Subcommand)]
pub enum SemanticGraphCommand {
    #[command(name = INSTALL_COMMAND_NAME)]
    Install(InstallArgs),
}
