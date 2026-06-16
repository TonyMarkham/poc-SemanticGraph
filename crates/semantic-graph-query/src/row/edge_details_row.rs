use crate::{
    QueryResult,
    model::{EdgeDetails, EdgeEndpoint, EdgeEvidence},
    sqlite::parse_json_value,
};

use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub(crate) struct EdgeDetailsRow {
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
    pub(crate) valid_to_run_id: Option<i64>,
    pub(crate) properties_json: String,
}

impl EdgeDetailsRow {
    pub(crate) fn into_model(
        self,
        source: EdgeEndpoint,
        target: EdgeEndpoint,
        evidence: Vec<EdgeEvidence>,
    ) -> QueryResult<EdgeDetails> {
        Ok(EdgeDetails {
            edge_id: self.edge_id,
            relation: self.relation,
            context: self.context,
            confidence: self.confidence,
            confidence_score: self.confidence_score,
            weight: self.weight,
            first_seen_run_id: self.first_seen_run_id,
            last_seen_run_id: self.last_seen_run_id,
            valid_to_run_id: self.valid_to_run_id,
            properties_json: parse_json_value(&self.properties_json)?,
            source,
            target,
            evidence,
        })
    }
}
