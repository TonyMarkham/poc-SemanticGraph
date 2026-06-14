use crate::dto::{GraphEdgeDto, GraphMetadataDto, GraphNodeDto};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GraphProjectionDto {
    pub nodes: Vec<GraphNodeDto>,
    pub edges: Vec<GraphEdgeDto>,
    pub metadata: GraphMetadataDto,
}
