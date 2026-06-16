use crate::model::{EdgeSummary, NodeSummary, ProjectionMetadata};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GraphProjection {
    pub nodes: Vec<NodeSummary>,
    pub edges: Vec<EdgeSummary>,
    pub metadata: ProjectionMetadata,
}
