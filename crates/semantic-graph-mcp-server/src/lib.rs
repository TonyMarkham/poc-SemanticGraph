mod args;
mod error;
mod resources;
mod rmcp_integration;
mod sanitize;
mod server;
mod tools;

pub use crate::{
    args::{ResolvedServerConfig, ServerArgs, resolve_server_config},
    error::{McpServerError, McpServerResult},
    rmcp_integration::serve_stdio,
    server::SemanticGraphMcpServer,
};

use clap::Parser;

pub async fn run_from_env() -> McpServerResult<()> {
    let args = ServerArgs::parse();
    run_with_args(args).await
}

pub async fn run_with_args(args: ServerArgs) -> McpServerResult<()> {
    let resolved = resolve_server_config(&args)?;
    let state = server::ServerState::from_resolved_config(resolved);
    serve_stdio(state).await
}
