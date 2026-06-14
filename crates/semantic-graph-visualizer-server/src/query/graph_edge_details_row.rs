use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub(crate) struct GraphEdgeDetailsRow {
    pub(crate) edge_id: String,
    pub(crate) source_node_id: String,
    pub(crate) target_node_id: String,
    pub(crate) relation: String,
    pub(crate) context: Option<String>,
    pub(crate) confidence: String,
    pub(crate) confidence_score: f64,
    pub(crate) weight: f64,
    pub(crate) first_seen_run_id: Option<i64>,
    pub(crate) last_seen_run_id: Option<i64>,
    pub(crate) properties_json: String,
}
