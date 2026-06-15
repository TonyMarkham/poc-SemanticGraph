use crate::FileInput;

use serde_json::Value;

#[derive(Debug, Clone)]
pub(crate) struct OwnedFileInput {
    pub(crate) workspace_id: i64,
    pub(crate) uri: String,
    pub(crate) path: String,
    pub(crate) language: String,
    pub(crate) content_hash: Option<String>,
    pub(crate) last_seen_run_id: Option<i64>,
    pub(crate) properties_json: Value,
}

impl From<FileInput<'_>> for OwnedFileInput {
    fn from(input: FileInput<'_>) -> Self {
        Self {
            workspace_id: input.workspace_id,
            uri: input.uri.to_string(),
            path: input.path.to_string(),
            language: input.language.to_string(),
            content_hash: input.content_hash.map(str::to_string),
            last_seen_run_id: input.last_seen_run_id,
            properties_json: input.properties_json,
        }
    }
}
