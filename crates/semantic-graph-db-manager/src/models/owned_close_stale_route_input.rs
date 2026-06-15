use crate::CloseStaleRouteInput;

#[derive(Debug, Clone)]
pub(crate) struct OwnedCloseStaleRouteInput {
    pub(crate) workspace_id: i64,
    pub(crate) run_id: i64,
    pub(crate) route: String,
    pub(crate) scope: String,
    pub(crate) scope_key: String,
    pub(crate) provider: String,
}

impl From<CloseStaleRouteInput<'_>> for OwnedCloseStaleRouteInput {
    fn from(input: CloseStaleRouteInput<'_>) -> Self {
        Self {
            workspace_id: input.workspace_id,
            run_id: input.run_id,
            route: input.route.to_string(),
            scope: input.scope.to_string(),
            scope_key: input.scope_key.to_string(),
            provider: input.provider.to_string(),
        }
    }
}
