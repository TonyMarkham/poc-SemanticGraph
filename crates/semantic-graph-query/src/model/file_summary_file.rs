use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FileSummaryFile {
    pub file_id: i64,
    pub uri: String,
    pub path: String,
    pub language: String,
    pub content_hash: Option<String>,
    pub last_seen_run_id: Option<i64>,
    pub properties_json: Value,
}
