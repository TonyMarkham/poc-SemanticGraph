use crate::TextRange;

use serde_json::Value;

#[derive(Debug, Clone)]
pub struct RouteWriteBatchEdgeEvidenceInput {
    pub edge_id: String,
    pub run_id: i64,
    pub provider: String,
    pub lsp_method: Option<String>,
    pub file_id: Option<i64>,
    pub range: Option<TextRange>,
    pub raw_json: Option<Value>,
}
