use serde_json::Value;

#[derive(Debug, Clone)]
pub(crate) struct OwnedFtsDocumentInput {
    pub(crate) workspace_id: i64,
    pub(crate) file_id: i64,
    pub(crate) path: String,
    pub(crate) language: String,
    pub(crate) content_hash: String,
    pub(crate) byte_len: i64,
    pub(crate) run_id: i64,
    pub(crate) content: String,
    pub(crate) properties_json: Value,
}
