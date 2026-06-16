use crate::model::{EdgeSummary, NodeSummary};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NodeNeighbor {
    pub direction: String,
    pub edge: EdgeSummary,
    pub adjacent_node: NodeSummary,
    pub relation: String,
    pub confidence: String,
}
