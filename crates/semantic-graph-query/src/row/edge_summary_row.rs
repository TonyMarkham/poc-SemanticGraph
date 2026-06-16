use crate::model::EdgeSummary;

use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub(crate) struct EdgeSummaryRow {
    pub(crate) edge_id: String,
    pub(crate) source_node_id: String,
    pub(crate) target_node_id: String,
    pub(crate) relation: String,
    pub(crate) context: Option<String>,
    pub(crate) confidence: String,
    pub(crate) confidence_score: f64,
    pub(crate) weight: f64,
    pub(crate) valid_to_run_id: Option<i64>,
}

impl EdgeSummaryRow {
    pub(crate) fn into_model(self) -> EdgeSummary {
        EdgeSummary {
            edge_id: self.edge_id,
            source_node_id: self.source_node_id,
            target_node_id: self.target_node_id,
            relation: self.relation,
            context: self.context,
            confidence: self.confidence,
            confidence_score: self.confidence_score,
            weight: self.weight,
            valid_to_run_id: self.valid_to_run_id,
        }
    }
}
