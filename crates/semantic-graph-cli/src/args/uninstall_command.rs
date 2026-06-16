use crate::{args::CodexUninstallArgs, constants::cli::CODEX_COMMAND_NAME};
use clap::Subcommand;

#[derive(Clone, Debug, Subcommand)]
pub enum UninstallCommand {
    #[command(name = CODEX_COMMAND_NAME)]
    Codex(CodexUninstallArgs),
}
