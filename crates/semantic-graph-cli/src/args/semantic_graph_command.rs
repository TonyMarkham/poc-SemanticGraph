use crate::{
    args::{InstallArgs, UninstallArgs},
    constants::cli::{INSTALL_COMMAND_NAME, UNINSTALL_COMMAND_NAME},
};
use clap::Subcommand;

#[derive(Clone, Debug, Subcommand)]
pub enum SemanticGraphCommand {
    #[command(name = INSTALL_COMMAND_NAME)]
    Install(InstallArgs),

    #[command(name = UNINSTALL_COMMAND_NAME)]
    Uninstall(UninstallArgs),
}
