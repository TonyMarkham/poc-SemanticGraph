use crate::model::{EdgeEndpoint, EdgeEvidence};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EdgeDetails {
    pub edge_id: String,
    pub relation: String,
    pub context: Option<String>,
    pub confidence: String,
    pub confidence_score: f64,
    pub weight: f64,
    pub first_seen_run_id: Option<i64>,
    pub last_seen_run_id: Option<i64>,
    pub valid_to_run_id: Option<i64>,
    pub properties_json: Value,
    pub source: EdgeEndpoint,
    pub target: EdgeEndpoint,
    pub evidence: Vec<EdgeEvidence>,
}
