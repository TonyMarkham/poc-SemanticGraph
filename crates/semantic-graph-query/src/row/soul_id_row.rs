use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub(crate) struct SoulIdRow {
    pub(crate) workspace_id: i64,
    pub(crate) root_uri: String,
    pub(crate) soul_id: String,
    pub(crate) has_document: i64,
    pub(crate) source_annotation_count: i64,
    pub(crate) linked_source_annotation_count: i64,
}
