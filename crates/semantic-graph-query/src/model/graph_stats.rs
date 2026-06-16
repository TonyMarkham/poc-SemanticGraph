use crate::model::ExtractionRunSummary;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GraphStats {
    pub workspace_count: i64,
    pub file_count: i64,
    pub active_node_count: i64,
    pub stale_node_count: i64,
    pub active_edge_count: i64,
    pub stale_edge_count: i64,
    pub occurrence_count: i64,
    pub edge_evidence_count: i64,
    pub route_status_count: i64,
    pub latest_runs: Vec<ExtractionRunSummary>,
}
