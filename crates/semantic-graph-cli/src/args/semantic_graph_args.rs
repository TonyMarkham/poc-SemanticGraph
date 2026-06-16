use crate::{args::SemanticGraphCommand, constants::cli::COMMAND_NAME};
use clap::Parser;

#[derive(Clone, Debug, Parser)]
#[command(name = COMMAND_NAME)]
#[command(about = "SemanticGraph project tooling")]
pub struct SemanticGraphArgs {
    #[command(subcommand)]
    pub command: SemanticGraphCommand,
}
