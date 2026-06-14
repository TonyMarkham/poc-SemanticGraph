use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub(crate) struct GraphEdgeEndpointRow {
    pub(crate) node_id: String,
    pub(crate) kind: String,
    pub(crate) display_label: String,
    pub(crate) qualified_name: Option<String>,
    pub(crate) language: String,
    pub(crate) source_file_path: Option<String>,
}
