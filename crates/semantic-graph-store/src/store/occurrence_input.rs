use crate::TextRange;

use serde_json::Value;

#[derive(Debug, Clone)]
pub struct OccurrenceInput<'a> {
    pub node_id: &'a str,
    pub run_id: i64,
    pub file_id: i64,
    pub role: &'a str,
    pub range: TextRange,
    pub enclosing_node_id: Option<&'a str>,
    pub raw_json: Option<Value>,
}
