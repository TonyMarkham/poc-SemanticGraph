use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub(crate) struct GraphEdgeEvidenceRow {
    pub(crate) id: i64,
    pub(crate) run_id: i64,
    pub(crate) provider: String,
    pub(crate) lsp_method: Option<String>,
    pub(crate) source_file_path: Option<String>,
    pub(crate) start_line: Option<i64>,
    pub(crate) start_col: Option<i64>,
    pub(crate) end_line: Option<i64>,
    pub(crate) end_col: Option<i64>,
    pub(crate) raw_json: Option<String>,
}
