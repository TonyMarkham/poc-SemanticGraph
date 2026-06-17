use crate::TextRange;

use serde_json::Value;

#[derive(Debug, Clone)]
pub struct DocumentSymbolWriteBatchNodeInput {
    pub workspace_id: i64,
    pub language: String,
    pub kind: String,
    pub name: String,
    pub qualified_name: Option<String>,
    pub display_name: Option<String>,
    pub symbol_key: String,
    pub file_uri: Option<String>,
    pub range: Option<TextRange>,
    pub selection_range: Option<TextRange>,
    pub container_node_id: Option<String>,
    pub properties_json: Value,
    pub run_id: Option<i64>,
}
