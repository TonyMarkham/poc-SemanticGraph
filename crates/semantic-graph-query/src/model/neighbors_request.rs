use crate::model::NeighborDirection;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NeighborsRequest {
    pub node_id: String,
    pub direction: Option<NeighborDirection>,
    pub relation: Option<String>,
    pub limit: Option<i64>,
}
