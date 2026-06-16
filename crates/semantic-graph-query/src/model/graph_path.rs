use crate::model::{GraphPathStep, NodeSummary};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GraphPath {
    pub source_node_id: String,
    pub target_node_id: String,
    pub found: bool,
    pub nodes: Vec<NodeSummary>,
    pub steps: Vec<GraphPathStep>,
    pub max_depth: i64,
    pub max_visited_nodes: i64,
}
