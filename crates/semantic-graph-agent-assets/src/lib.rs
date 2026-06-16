mod args;
mod command_output;
pub mod constants;
mod error;
mod files;
mod manifest;
mod render;

pub use crate::{
    args::{AgentAssetsArgs, AgentAssetsCommand},
    command_output::CommandOutput,
    error::{AgentAssetsError, AgentAssetsResult},
    files::{AssetWriteReport, DriftCheckReport},
    manifest::AssetManifest,
    render::{AssetRenderer, RenderedAsset},
};

use clap::Parser;
use std::{env, path::Path};

pub fn run_from_env() -> AgentAssetsResult<CommandOutput> {
    let args = AgentAssetsArgs::parse();
    let repo_root = env::current_dir()
        .map_err(|source| AgentAssetsError::io("resolve current directory", None, source))?;
    run_with_args(args, &repo_root)
}

pub fn run_with_args(args: AgentAssetsArgs, repo_root: &Path) -> AgentAssetsResult<CommandOutput> {
    match args.command {
        AgentAssetsCommand::Generate => generate_assets(repo_root).map(CommandOutput::Generate),
        AgentAssetsCommand::Check => check_assets(repo_root).map(CommandOutput::Check),
    }
}

pub fn generate_assets(repo_root: &Path) -> AgentAssetsResult<AssetWriteReport> {
    let manifest = AssetManifest::load_from_repo(repo_root)?;
    let assets = AssetRenderer::render(repo_root, &manifest)?;
    files::AssetWriter::write(repo_root, &manifest, &assets)
}

pub fn check_assets(repo_root: &Path) -> AgentAssetsResult<DriftCheckReport> {
    let manifest = AssetManifest::load_from_repo(repo_root)?;
    let assets = AssetRenderer::render(repo_root, &manifest)?;
    files::DriftChecker::check(repo_root, &manifest, &assets)
}
