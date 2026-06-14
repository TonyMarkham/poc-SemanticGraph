use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub(crate) struct GraphNodeRelationSummaryRow {
    pub(crate) direction: String,
    pub(crate) relation: String,
    pub(crate) edge_count: i64,
}
