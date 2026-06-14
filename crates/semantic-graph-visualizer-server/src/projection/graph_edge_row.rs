use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub(crate) struct GraphEdgeRow {
    pub(crate) id: String,
    pub(crate) source_node_id: String,
    pub(crate) target_node_id: String,
    pub(crate) relation: String,
    pub(crate) confidence: String,
    pub(crate) confidence_score: f64,
}
