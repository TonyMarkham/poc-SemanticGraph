use crate::{args::AgentAssetsCommand, constants::cli::COMMAND_NAME};
use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = COMMAND_NAME)]
#[command(about = "Generate and check SemanticGraph Codex asset snapshots")]
pub struct AgentAssetsArgs {
    #[command(subcommand)]
    pub command: AgentAssetsCommand,
}
