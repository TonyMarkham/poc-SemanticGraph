use crate::dto::{GraphNodeOccurrenceDto, GraphNodeRelationSummaryDto};

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodeDetailsDto {
    pub node_id: String,
    pub kind: String,
    pub name: String,
    pub display_label: String,
    pub qualified_name: Option<String>,
    pub language: String,
    pub source_file_path: Option<String>,
    pub start_line: Option<i64>,
    pub start_col: Option<i64>,
    pub end_line: Option<i64>,
    pub end_col: Option<i64>,
    pub selection_start_line: Option<i64>,
    pub selection_start_col: Option<i64>,
    pub container_node_id: Option<String>,
    pub container_display_label: Option<String>,
    pub first_seen_run_id: Option<i64>,
    pub last_seen_run_id: Option<i64>,
    pub properties_json: Value,
    pub incoming_edge_count: i64,
    pub outgoing_edge_count: i64,
    pub relations: Vec<GraphNodeRelationSummaryDto>,
    pub occurrences: Vec<GraphNodeOccurrenceDto>,
}
