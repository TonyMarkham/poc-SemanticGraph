use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EdgeSummary {
    pub edge_id: String,
    pub source_node_id: String,
    pub target_node_id: String,
    pub relation: String,
    pub context: Option<String>,
    pub confidence: String,
    pub confidence_score: f64,
    pub weight: f64,
    pub valid_to_run_id: Option<i64>,
}
