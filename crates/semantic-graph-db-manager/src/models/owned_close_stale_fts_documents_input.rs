use crate::CloseStaleFtsDocumentsInput;

#[derive(Debug, Clone)]
pub(crate) struct OwnedCloseStaleFtsDocumentsInput {
    pub(crate) workspace_id: i64,
    pub(crate) run_id: i64,
    pub(crate) provider: String,
    pub(crate) route: String,
    pub(crate) scope: String,
    pub(crate) scope_key: String,
}

impl From<CloseStaleFtsDocumentsInput<'_>> for OwnedCloseStaleFtsDocumentsInput {
    fn from(input: CloseStaleFtsDocumentsInput<'_>) -> Self {
        Self {
            workspace_id: input.workspace_id,
            run_id: input.run_id,
            provider: input.provider.to_string(),
            route: input.route.to_string(),
            scope: input.scope.to_string(),
            scope_key: input.scope_key.to_string(),
        }
    }
}
