use schemars::JsonSchema;
use semantic_graph_query::SoulSearchRequest;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SoulSearchParams {
    pub workspace_id: Option<i64>,
    pub root_uri: Option<String>,
    pub query: Option<String>,
    pub include_markdown_sources: Option<bool>,
    pub include_source_annotations: Option<bool>,
    pub coverage: Option<String>,
    pub limit: Option<i64>,
    pub cursor: Option<String>,
}

impl From<SoulSearchParams> for SoulSearchRequest {
    fn from(value: SoulSearchParams) -> Self {
        Self {
            workspace_id: value.workspace_id,
            root_uri: value.root_uri,
            query: value.query,
            include_markdown_sources: value.include_markdown_sources,
            include_source_annotations: value.include_source_annotations,
            coverage: value.coverage,
            limit: value.limit,
            cursor: value.cursor,
        }
    }
}
