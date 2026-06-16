use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExtractionRunSummary {
    pub run_id: i64,
    pub workspace_id: i64,
    pub root_uri: String,
    pub provider: String,
    pub provider_version: Option<String>,
    pub git_commit: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub status: String,
    pub properties_json: Value,
}
