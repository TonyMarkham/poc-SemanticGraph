use crate::{
    model::{EdgeSummary, NodeSummary, SoulLinkedSource},
    row::EdgeSummaryRow,
};

use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub(crate) struct SoulLinkedSourceRow {
    pub(crate) source_node_id: String,
    pub(crate) source_kind: String,
    pub(crate) source_name: String,
    pub(crate) source_display_label: String,
    pub(crate) source_qualified_name: Option<String>,
    pub(crate) source_language: String,
    pub(crate) source_file_path: Option<String>,
    pub(crate) source_valid_to_run_id: Option<i64>,
    pub(crate) source_start_line: Option<i64>,
    pub(crate) source_start_col: Option<i64>,
    pub(crate) source_end_line: Option<i64>,
    pub(crate) source_end_col: Option<i64>,
    pub(crate) edge_id: Option<String>,
    pub(crate) edge_source_node_id: Option<String>,
    pub(crate) edge_target_node_id: Option<String>,
    pub(crate) edge_relation: Option<String>,
    pub(crate) edge_context: Option<String>,
    pub(crate) edge_confidence: Option<String>,
    pub(crate) edge_confidence_score: Option<f64>,
    pub(crate) edge_weight: Option<f64>,
    pub(crate) edge_valid_to_run_id: Option<i64>,
}

impl SoulLinkedSourceRow {
    pub(crate) fn into_model(self) -> SoulLinkedSource {
        let source_file_language = infer_source_file_language(self.source_file_path.as_deref());
        let edge = self.edge_summary();

        SoulLinkedSource {
            source: NodeSummary {
                node_id: self.source_node_id,
                kind: self.source_kind,
                name: self.source_name,
                display_label: self.source_display_label,
                qualified_name: self.source_qualified_name,
                language: self.source_language,
                source_file_path: self.source_file_path,
                valid_to_run_id: self.source_valid_to_run_id,
            },
            edge,
            source_file_language,
            start_line: self.source_start_line,
            start_col: self.source_start_col,
            end_line: self.source_end_line,
            end_col: self.source_end_col,
        }
    }

    fn edge_summary(&self) -> Option<EdgeSummary> {
        let edge_id = self.edge_id.clone()?;

        Some(
            EdgeSummaryRow {
                edge_id,
                source_node_id: self.edge_source_node_id.clone()?,
                target_node_id: self.edge_target_node_id.clone()?,
                relation: self.edge_relation.clone()?,
                context: self.edge_context.clone(),
                confidence: self.edge_confidence.clone()?,
                confidence_score: self.edge_confidence_score?,
                weight: self.edge_weight?,
                valid_to_run_id: self.edge_valid_to_run_id,
            }
            .into_model(),
        )
    }
}

fn infer_source_file_language(path: Option<&str>) -> String {
    let Some(path) = path else {
        return "unknown".to_string();
    };

    if path.ends_with(".rs") {
        "rust".to_string()
    } else if path.ends_with(".cs") {
        "csharp".to_string()
    } else if path.ends_with(".md") || path.ends_with(".markdown") {
        "markdown".to_string()
    } else {
        "other".to_string()
    }
}
