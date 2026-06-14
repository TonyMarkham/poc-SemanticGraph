use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GraphMetadataDto {
    pub database_path: String,
    pub limit: i64,
    pub node_count: usize,
    pub edge_count: usize,
}
