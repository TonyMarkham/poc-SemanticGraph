use crate::{
    QueryResult,
    model::{NodeDetails, NodeOccurrence, NodeRelationSummary, NodeSummary},
    sqlite::parse_json_value,
};

use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub(crate) struct NodeDetailsRow {
    pub(crate) node_id: String,
    pub(crate) kind: String,
    pub(crate) name: String,
    pub(crate) display_label: String,
    pub(crate) qualified_name: Option<String>,
    pub(crate) language: String,
    pub(crate) source_file_path: Option<String>,
    pub(crate) start_line: Option<i64>,
    pub(crate) start_col: Option<i64>,
    pub(crate) end_line: Option<i64>,
    pub(crate) end_col: Option<i64>,
    pub(crate) selection_start_line: Option<i64>,
    pub(crate) selection_start_col: Option<i64>,
    pub(crate) container_node_id: Option<String>,
    pub(crate) container_kind: Option<String>,
    pub(crate) container_name: Option<String>,
    pub(crate) container_display_label: Option<String>,
    pub(crate) container_qualified_name: Option<String>,
    pub(crate) container_language: Option<String>,
    pub(crate) container_source_file_path: Option<String>,
    pub(crate) container_valid_to_run_id: Option<i64>,
    pub(crate) first_seen_run_id: Option<i64>,
    pub(crate) last_seen_run_id: Option<i64>,
    pub(crate) valid_to_run_id: Option<i64>,
    pub(crate) properties_json: String,
    pub(crate) incoming_edge_count: i64,
    pub(crate) outgoing_edge_count: i64,
}

impl NodeDetailsRow {
    pub(crate) fn into_model(
        self,
        relations: Vec<NodeRelationSummary>,
        occurrences: Vec<NodeOccurrence>,
    ) -> QueryResult<NodeDetails> {
        let container = self.container_summary();
        let properties_json = parse_json_value(&self.properties_json)?;

        Ok(NodeDetails {
            node_id: self.node_id,
            kind: self.kind,
            name: self.name,
            display_label: self.display_label,
            qualified_name: self.qualified_name,
            language: self.language,
            source_file_path: self.source_file_path,
            start_line: self.start_line,
            start_col: self.start_col,
            end_line: self.end_line,
            end_col: self.end_col,
            selection_start_line: self.selection_start_line,
            selection_start_col: self.selection_start_col,
            container,
            first_seen_run_id: self.first_seen_run_id,
            last_seen_run_id: self.last_seen_run_id,
            valid_to_run_id: self.valid_to_run_id,
            properties_json,
            incoming_edge_count: self.incoming_edge_count,
            outgoing_edge_count: self.outgoing_edge_count,
            relations,
            occurrences,
        })
    }

    fn container_summary(&self) -> Option<NodeSummary> {
        let node_id = self.container_node_id.clone()?;

        Some(NodeSummary {
            node_id,
            kind: self.container_kind.clone()?,
            name: self.container_name.clone()?,
            display_label: self.container_display_label.clone()?,
            qualified_name: self.container_qualified_name.clone(),
            language: self.container_language.clone()?,
            source_file_path: self.container_source_file_path.clone(),
            valid_to_run_id: self.container_valid_to_run_id,
        })
    }
}
