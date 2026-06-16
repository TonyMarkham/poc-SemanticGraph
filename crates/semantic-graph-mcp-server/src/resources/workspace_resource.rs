use crate::{
    rmcp_integration::query_error_to_mcp,
    sanitize::{DEFAULT_TEXT_CAP, sanitize_transcript_text},
    server::ServerState,
};

use rmcp::ErrorData;

pub const WORKSPACE_RESOURCE_URI: &str = "semantic-graph://workspace";

pub async fn workspace_resource_text(state: &ServerState) -> Result<String, ErrorData> {
    let stats = state
        .query_service()
        .stats()
        .await
        .map_err(query_error_to_mcp)?;
    let latest_runs = stats
        .latest_runs
        .iter()
        .map(|run| {
            format!(
                "- run {} workspace {} status {} provider {} started {}",
                run.run_id, run.workspace_id, run.status, run.provider, run.started_at
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let latest_runs = if latest_runs.is_empty() {
        "- none".to_string()
    } else {
        latest_runs
    };

    Ok(sanitize_transcript_text(
        &format!(
            "SemanticGraph MCP server context.\n\nDatabase path: {}\nTransport: stdio-only\nMode: read-only\nWorkspace count: {}\nFiles: {}\nActive nodes: {}\nActive edges: {}\nRoute status rows: {}\n\nLatest extraction runs:\n{}",
            state.database_path().display(),
            stats.workspace_count,
            stats.file_count,
            stats.active_node_count,
            stats.active_edge_count,
            stats.route_status_count,
            latest_runs
        ),
        DEFAULT_TEXT_CAP * 8,
    ))
}
