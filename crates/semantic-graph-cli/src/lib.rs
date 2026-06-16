mod args;
mod codex_config;
mod command_output;
mod constants;
mod error;
mod install;

pub use crate::{
    args::{
        CodexInstallArgs, InstallArgs, InstallCommand, McpInstallMode, SemanticGraphArgs,
        SemanticGraphCommand,
    },
    command_output::CommandOutput,
    error::{SemanticGraphCliError, SemanticGraphCliResult},
    install::{
        AssetSource, Checksum, CodexInstallPlan, CodexInstallReport, CodexInstaller, FileAction,
        FileActionKind, InstallManifest, InstallManifestMode, ManagedFile,
        ManagedFileManifestEntry, ProjectRoot,
    },
};

use clap::Parser;
use std::env;

pub fn run_from_env() -> SemanticGraphCliResult<CommandOutput> {
    let args = SemanticGraphArgs::parse();
    run_with_args(args)
}

pub fn run_with_args(args: SemanticGraphArgs) -> SemanticGraphCliResult<CommandOutput> {
    match args.command {
        SemanticGraphCommand::Install(install_args) => match install_args.command {
            InstallCommand::Codex(codex_args) => {
                let current_dir = env::current_dir().map_err(|source| {
                    SemanticGraphCliError::io("resolve current directory", None, source)
                })?;
                let repo_root = workspace_repo_root();
                CodexInstaller::new(repo_root).install(&codex_args, &current_dir)
            }
        },
    }
    .map(CommandOutput::InstallCodex)
}

fn workspace_repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}
