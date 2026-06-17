use crate::FtsDocumentInput;

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

impl From<FtsDocumentInput<'_>> for OwnedFtsDocumentInput {
    fn from(input: FtsDocumentInput<'_>) -> Self {
        Self {
            workspace_id: input.workspace_id,
            file_id: input.file_id,
            path: input.path.to_string(),
            language: input.language.to_string(),
            content_hash: input.content_hash.to_string(),
            byte_len: input.byte_len,
            run_id: input.run_id,
            content: input.content.to_string(),
            properties_json: input.properties_json,
        }
    }
}
