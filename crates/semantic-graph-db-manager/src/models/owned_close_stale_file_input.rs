use crate::CloseStaleFileInput;

#[derive(Debug, Clone)]
pub(crate) struct OwnedCloseStaleFileInput {
    pub(crate) workspace_id: i64,
    pub(crate) run_id: i64,
    pub(crate) file_uri: String,
}

impl From<CloseStaleFileInput<'_>> for OwnedCloseStaleFileInput {
    fn from(input: CloseStaleFileInput<'_>) -> Self {
        Self {
            workspace_id: input.workspace_id,
            run_id: input.run_id,
            file_uri: input.file_uri.to_string(),
        }
    }
}
