use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileSummaryRequest {
    pub workspace_id: Option<i64>,
    pub root_uri: Option<String>,
    pub file_path: String,
    pub edge_limit: Option<i64>,
}
