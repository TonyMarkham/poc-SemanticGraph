use crate::EdgeInput;

use serde_json::Value;

#[derive(Debug, Clone)]
pub(crate) struct OwnedEdgeInput {
    pub(crate) workspace_id: i64,
    pub(crate) src_node_id: String,
    pub(crate) dst_node_id: String,
    pub(crate) relation: String,
    pub(crate) context: Option<String>,
    pub(crate) confidence: String,
    pub(crate) confidence_score: f64,
    pub(crate) weight: f64,
    pub(crate) properties_json: Value,
    pub(crate) run_id: Option<i64>,
}

impl From<EdgeInput<'_>> for OwnedEdgeInput {
    fn from(input: EdgeInput<'_>) -> Self {
        Self {
            workspace_id: input.workspace_id,
            src_node_id: input.src_node_id.to_string(),
            dst_node_id: input.dst_node_id.to_string(),
            relation: input.relation.to_string(),
            context: input.context.map(str::to_string),
            confidence: input.confidence.to_string(),
            confidence_score: input.confidence_score,
            weight: input.weight,
            properties_json: input.properties_json,
            run_id: input.run_id,
        }
    }
}
