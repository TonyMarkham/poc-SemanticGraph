#[derive(Debug, sqlx::FromRow)]
pub(crate) struct FtsDocumentContentRow {
    pub(crate) content: String,
}
