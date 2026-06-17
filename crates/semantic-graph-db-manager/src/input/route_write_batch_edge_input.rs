use serde_json::Value;

#[derive(Debug, Clone)]
pub struct RouteWriteBatchEdgeInput {
    pub workspace_id: i64,
    pub src_node_id: String,
    pub dst_node_id: String,
    pub relation: String,
    pub context: Option<String>,
    pub confidence: String,
    pub confidence_score: f64,
    pub weight: f64,
    pub properties_json: Value,
    pub run_id: Option<i64>,
}
