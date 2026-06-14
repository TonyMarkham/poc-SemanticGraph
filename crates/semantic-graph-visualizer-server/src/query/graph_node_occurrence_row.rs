use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub(crate) struct GraphNodeOccurrenceRow {
    pub(crate) id: i64,
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
