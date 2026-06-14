use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub(crate) struct GraphNodeDetailsRow {
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
    pub(crate) container_display_label: Option<String>,
    pub(crate) first_seen_run_id: Option<i64>,
    pub(crate) last_seen_run_id: Option<i64>,
    pub(crate) properties_json: String,
    pub(crate) incoming_edge_count: i64,
    pub(crate) outgoing_edge_count: i64,
}
