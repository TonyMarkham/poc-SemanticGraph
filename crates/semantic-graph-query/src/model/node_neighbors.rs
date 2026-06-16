use crate::model::{NodeNeighbor, NodeSummary};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NodeNeighbors {
    pub node: NodeSummary,
    pub incoming: Vec<NodeNeighbor>,
    pub outgoing: Vec<NodeNeighbor>,
    pub requested_limit: Option<i64>,
    pub applied_limit: i64,
}
