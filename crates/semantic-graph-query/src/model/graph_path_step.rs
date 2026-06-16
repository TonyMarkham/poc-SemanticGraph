use crate::model::{EdgeSummary, NodeSummary};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GraphPathStep {
    pub edge: EdgeSummary,
    pub node: NodeSummary,
}
