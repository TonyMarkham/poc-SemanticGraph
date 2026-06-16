use schemars::JsonSchema;
use semantic_graph_query::FileSummaryRequest;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileSummaryParams {
    pub workspace_id: Option<i64>,
    pub root_uri: Option<String>,
    pub file_path: String,
    pub edge_limit: Option<i64>,
}

impl From<FileSummaryParams> for FileSummaryRequest {
    fn from(value: FileSummaryParams) -> Self {
        Self {
            workspace_id: value.workspace_id,
            root_uri: value.root_uri,
            file_path: value.file_path,
            edge_limit: value.edge_limit,
        }
    }
}
