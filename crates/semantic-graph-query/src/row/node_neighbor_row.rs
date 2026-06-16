use crate::model::{EdgeSummary, NodeNeighbor, NodeSummary};

use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub(crate) struct NodeNeighborRow {
    pub(crate) direction: String,
    pub(crate) edge_id: String,
    pub(crate) source_node_id: String,
    pub(crate) target_node_id: String,
    pub(crate) relation: String,
    pub(crate) context: Option<String>,
    pub(crate) confidence: String,
    pub(crate) confidence_score: f64,
    pub(crate) weight: f64,
    pub(crate) edge_valid_to_run_id: Option<i64>,
    pub(crate) adjacent_node_id: String,
    pub(crate) adjacent_kind: String,
    pub(crate) adjacent_name: String,
    pub(crate) adjacent_display_label: String,
    pub(crate) adjacent_qualified_name: Option<String>,
    pub(crate) adjacent_language: String,
    pub(crate) adjacent_source_file_path: Option<String>,
    pub(crate) adjacent_valid_to_run_id: Option<i64>,
}

impl NodeNeighborRow {
    pub(crate) fn edge_summary(&self) -> EdgeSummary {
        EdgeSummary {
            edge_id: self.edge_id.clone(),
            source_node_id: self.source_node_id.clone(),
            target_node_id: self.target_node_id.clone(),
            relation: self.relation.clone(),
            context: self.context.clone(),
            confidence: self.confidence.clone(),
            confidence_score: self.confidence_score,
            weight: self.weight,
            valid_to_run_id: self.edge_valid_to_run_id,
        }
    }

    pub(crate) fn adjacent_node_summary(&self) -> NodeSummary {
        NodeSummary {
            node_id: self.adjacent_node_id.clone(),
            kind: self.adjacent_kind.clone(),
            name: self.adjacent_name.clone(),
            display_label: self.adjacent_display_label.clone(),
            qualified_name: self.adjacent_qualified_name.clone(),
            language: self.adjacent_language.clone(),
            source_file_path: self.adjacent_source_file_path.clone(),
            valid_to_run_id: self.adjacent_valid_to_run_id,
        }
    }

    pub(crate) fn into_model(self) -> NodeNeighbor {
        NodeNeighbor {
            direction: self.direction.clone(),
            edge: self.edge_summary(),
            adjacent_node: self.adjacent_node_summary(),
            relation: self.relation,
            confidence: self.confidence,
        }
    }
}
