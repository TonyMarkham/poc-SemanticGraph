use crate::{args::CodexInstallArgs, constants::cli::CODEX_COMMAND_NAME};
use clap::Subcommand;

#[derive(Clone, Debug, Subcommand)]
pub enum InstallCommand {
    #[command(name = CODEX_COMMAND_NAME)]
    Codex(CodexInstallArgs),
}
