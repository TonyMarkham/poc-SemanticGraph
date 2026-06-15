use crate::TextRange;

use serde_json::Value;

#[derive(Debug, Clone)]
pub struct EdgeEvidenceInput<'a> {
    pub edge_id: &'a str,
    pub run_id: i64,
    pub provider: &'a str,
    pub lsp_method: Option<&'a str>,
    pub file_id: Option<i64>,
    pub range: Option<TextRange>,
    pub raw_json: Option<Value>,
}
