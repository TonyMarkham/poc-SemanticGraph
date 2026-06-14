use semantic_graph_store::TextRange;

use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct ReferenceOccurrence {
    pub file_uri: String,
    pub file_relative_path: String,
    pub file_symbol_key: String,
    pub range: TextRange,
    pub enclosing_symbol_key: Option<String>,
    pub raw_json: Value,
}
