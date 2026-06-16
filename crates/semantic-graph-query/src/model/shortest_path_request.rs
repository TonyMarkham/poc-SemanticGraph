use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShortestPathRequest {
    pub source_node_id: String,
    pub target_node_id: String,
    pub max_depth: Option<i64>,
    pub max_visited_nodes: Option<i64>,
}
