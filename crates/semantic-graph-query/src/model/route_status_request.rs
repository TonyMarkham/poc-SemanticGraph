use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RouteStatusRequest {
    pub workspace_id: Option<i64>,
    pub root_uri: Option<String>,
    pub route: Option<String>,
    pub scope: Option<String>,
    pub scope_key: Option<String>,
    pub file_path: Option<String>,
    pub limit: Option<i64>,
}
