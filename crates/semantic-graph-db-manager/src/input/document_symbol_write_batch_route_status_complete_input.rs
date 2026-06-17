use serde_json::Value;

#[derive(Debug, Clone)]
pub struct DocumentSymbolWriteBatchRouteStatusCompleteInput {
    pub workspace_id: i64,
    pub route: String,
    pub scope: String,
    pub scope_key: String,
    pub provider: String,
    pub provider_version: Option<String>,
    pub content_hash: Option<String>,
    pub run_id: i64,
    pub diagnostics_json: Value,
}
