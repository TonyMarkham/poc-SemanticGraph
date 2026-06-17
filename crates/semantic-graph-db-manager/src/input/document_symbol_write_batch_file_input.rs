use serde_json::Value;

#[derive(Debug, Clone)]
pub struct DocumentSymbolWriteBatchFileInput {
    pub workspace_id: i64,
    pub uri: String,
    pub path: String,
    pub language: String,
    pub content_hash: Option<String>,
    pub last_seen_run_id: Option<i64>,
    pub properties_json: Value,
}
