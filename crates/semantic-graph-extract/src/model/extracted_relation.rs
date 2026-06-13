use semantic_graph_store::TextRange;
use serde_json::Value;

use crate::model::ProviderId;

#[derive(Debug, Clone, PartialEq)]
pub struct ExtractedRelation {
    pub provider: ProviderId,
    pub source_symbol_key: String,
    pub target_symbol_key: String,
    pub relation: String,
    pub confidence: String,
    pub confidence_score: f64,
    pub range: Option<TextRange>,
    pub raw_json: Value,
}
