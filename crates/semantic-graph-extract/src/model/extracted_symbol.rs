use semantic_graph_store::TextRange;
use serde_json::Value;

use crate::model::{GraphLanguage, ProviderId};

#[derive(Debug, Clone, PartialEq)]
pub struct ExtractedSymbol {
    pub provider: ProviderId,
    pub language: GraphLanguage,
    pub file_uri: String,
    pub symbol_key: String,
    pub parent_symbol_key: Option<String>,
    pub name: String,
    pub kind: String,
    pub qualified_name: Option<String>,
    pub detail: Option<String>,
    pub range: TextRange,
    pub selection_range: TextRange,
    pub raw_json: Value,
}
