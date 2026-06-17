use serde_json::Value;

#[derive(Debug, Clone)]
pub struct DocumentSymbolWriteBatchObservationInput {
    pub workspace_id: i64,
    pub run_id: i64,
    pub route: String,
    pub scope: String,
    pub scope_key: String,
    pub provider: String,
    pub entity_kind: String,
    pub entity_id: String,
    pub source_file_uri: Option<String>,
    pub properties_json: Value,
}
