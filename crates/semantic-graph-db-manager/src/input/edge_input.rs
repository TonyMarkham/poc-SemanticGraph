use serde_json::Value;

#[derive(Debug, Clone)]
pub struct EdgeInput<'a> {
    pub workspace_id: i64,
    pub src_node_id: &'a str,
    pub dst_node_id: &'a str,
    pub relation: &'a str,
    pub context: Option<&'a str>,
    pub confidence: &'a str,
    pub confidence_score: f64,
    pub weight: f64,
    pub properties_json: Value,
    pub run_id: Option<i64>,
}
