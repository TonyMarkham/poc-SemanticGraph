use crate::{
    McpServerError, McpServerResult,
    server::{SemanticGraphMcpServer, ServerState},
};

use rmcp::{ServiceExt, transport::io::stdio};

pub async fn serve_stdio(state: ServerState) -> McpServerResult<()> {
    let service = SemanticGraphMcpServer::new(state)
        .serve(stdio())
        .await
        .map_err(|source| McpServerError::rmcp(source.into()))?;
    service
        .waiting()
        .await
        .map_err(|source| McpServerError::rmcp(source.into()))?;
    Ok(())
}
