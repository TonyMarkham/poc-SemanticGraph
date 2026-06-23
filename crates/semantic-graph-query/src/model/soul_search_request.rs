use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SoulSearchRequest {
    pub workspace_id: Option<i64>,
    pub root_uri: Option<String>,
    pub query: Option<String>,
    pub include_markdown_sources: Option<bool>,
    pub include_source_annotations: Option<bool>,
    pub coverage: Option<String>,
    pub limit: Option<i64>,
    pub cursor: Option<String>,
}
