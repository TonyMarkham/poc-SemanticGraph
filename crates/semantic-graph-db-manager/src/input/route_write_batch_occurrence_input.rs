use crate::TextRange;

use serde_json::Value;

#[derive(Debug, Clone)]
pub struct RouteWriteBatchOccurrenceInput {
    pub node_id: String,
    pub run_id: i64,
    pub file_id: i64,
    pub role: String,
    pub range: TextRange,
    pub enclosing_node_id: Option<String>,
    pub raw_json: Option<Value>,
}
