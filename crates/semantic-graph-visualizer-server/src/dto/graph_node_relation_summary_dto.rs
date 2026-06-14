use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodeRelationSummaryDto {
    pub direction: String,
    pub relation: String,
    pub edge_count: i64,
}
