use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RouteStatus {
    pub route_status_id: i64,
    pub workspace_id: i64,
    pub root_uri: String,
    pub route: String,
    pub scope: String,
    pub scope_key: String,
    pub file_path: Option<String>,
    pub provider: String,
    pub provider_version: Option<String>,
    pub content_hash: Option<String>,
    pub last_started_run_id: Option<i64>,
    pub last_complete_run_id: Option<i64>,
    pub last_status: String,
    pub diagnostics_json: Value,
    pub updated_at: String,
}
