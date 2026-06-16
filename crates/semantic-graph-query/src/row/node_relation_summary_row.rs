use crate::model::NodeRelationSummary;

use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub(crate) struct NodeRelationSummaryRow {
    pub(crate) direction: String,
    pub(crate) relation: String,
    pub(crate) edge_count: i64,
}

impl NodeRelationSummaryRow {
    pub(crate) fn into_model(self) -> NodeRelationSummary {
        NodeRelationSummary {
            direction: self.direction,
            relation: self.relation,
            edge_count: self.edge_count,
        }
    }
}
