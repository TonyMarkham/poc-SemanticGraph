use crate::constants::cli::{CHECK_COMMAND_NAME, GENERATE_COMMAND_NAME};
use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum AgentAssetsCommand {
    #[command(name = GENERATE_COMMAND_NAME)]
    Generate,

    #[command(name = CHECK_COMMAND_NAME)]
    Check,
}
