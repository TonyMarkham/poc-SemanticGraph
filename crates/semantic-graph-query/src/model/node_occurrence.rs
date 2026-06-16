use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NodeOccurrence {
    pub occurrence_id: i64,
    pub run_id: i64,
    pub role: String,
    pub source_file_path: String,
    pub start_line: i64,
    pub start_col: i64,
    pub end_line: i64,
    pub end_col: i64,
    pub enclosing_node_id: Option<String>,
    pub raw_json: Option<Value>,
}
