use schemars::JsonSchema;
use semantic_graph_query::ShortestPathRequest;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShortestPathParams {
    pub source_node_id: String,
    pub target_node_id: String,
    pub max_depth: Option<i64>,
    pub max_visited_nodes: Option<i64>,
}

impl From<ShortestPathParams> for ShortestPathRequest {
    fn from(value: ShortestPathParams) -> Self {
        Self {
            source_node_id: value.source_node_id,
            target_node_id: value.target_node_id,
            max_depth: value.max_depth,
            max_visited_nodes: value.max_visited_nodes,
        }
    }
}
