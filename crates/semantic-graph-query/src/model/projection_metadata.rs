use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionMetadata {
    pub database_path: String,
    pub requested_limit: Option<i64>,
    pub applied_limit: i64,
    pub node_count: usize,
    pub edge_count: usize,
}
