use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NodeRelationSummary {
    pub direction: String,
    pub relation: String,
    pub edge_count: i64,
}
