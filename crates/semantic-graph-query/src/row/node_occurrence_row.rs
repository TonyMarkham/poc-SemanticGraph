use crate::{QueryResult, model::NodeOccurrence, sqlite::parse_optional_json_value};

use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub(crate) struct NodeOccurrenceRow {
    pub(crate) occurrence_id: i64,
    pub(crate) run_id: i64,
    pub(crate) role: String,
    pub(crate) source_file_path: String,
    pub(crate) start_line: i64,
    pub(crate) start_col: i64,
    pub(crate) end_line: i64,
    pub(crate) end_col: i64,
    pub(crate) enclosing_node_id: Option<String>,
    pub(crate) raw_json: Option<String>,
}

impl NodeOccurrenceRow {
    pub(crate) fn into_model(self) -> QueryResult<NodeOccurrence> {
        Ok(NodeOccurrence {
            occurrence_id: self.occurrence_id,
            run_id: self.run_id,
            role: self.role,
            source_file_path: self.source_file_path,
            start_line: self.start_line,
            start_col: self.start_col,
            end_line: self.end_line,
            end_col: self.end_col,
            enclosing_node_id: self.enclosing_node_id,
            raw_json: parse_optional_json_value(self.raw_json)?,
        })
    }
}
