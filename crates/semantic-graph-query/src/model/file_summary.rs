use crate::model::{EdgeSummary, FileSummaryFile, NodeSummary, RouteStatus};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FileSummary {
    pub workspace_id: i64,
    pub root_uri: String,
    pub file: FileSummaryFile,
    pub file_node: Option<NodeSummary>,
    pub symbols: Vec<NodeSummary>,
    pub touching_edges: Vec<EdgeSummary>,
    pub file_route_statuses: Vec<RouteStatus>,
    pub workspace_route_statuses: Vec<RouteStatus>,
    pub requested_edge_limit: Option<i64>,
    pub applied_edge_limit: i64,
}
