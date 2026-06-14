use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdgeEvidenceDto {
    pub id: i64,
    pub run_id: i64,
    pub provider: String,
    pub lsp_method: Option<String>,
    pub source_file_path: Option<String>,
    pub start_line: Option<i64>,
    pub start_col: Option<i64>,
    pub end_line: Option<i64>,
    pub end_col: Option<i64>,
    pub raw_json: Option<Value>,
}
