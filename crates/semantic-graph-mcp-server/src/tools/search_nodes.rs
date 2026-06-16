use schemars::JsonSchema;
use semantic_graph_query::NodeSearchRequest;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SearchNodesParams {
    pub query: String,
    pub limit: Option<i64>,
}

impl From<SearchNodesParams> for NodeSearchRequest {
    fn from(value: SearchNodesParams) -> Self {
        Self {
            query: value.query,
            limit: value.limit,
        }
    }
}
