use crate::RouteStatusCompleteInput;

use serde_json::Value;

#[derive(Debug, Clone)]
pub(crate) struct OwnedRouteStatusCompleteInput {
    pub(crate) workspace_id: i64,
    pub(crate) route: String,
    pub(crate) scope: String,
    pub(crate) scope_key: String,
    pub(crate) provider: String,
    pub(crate) provider_version: Option<String>,
    pub(crate) content_hash: Option<String>,
    pub(crate) run_id: i64,
    pub(crate) diagnostics_json: Value,
}

impl From<RouteStatusCompleteInput<'_>> for OwnedRouteStatusCompleteInput {
    fn from(input: RouteStatusCompleteInput<'_>) -> Self {
        Self {
            workspace_id: input.workspace_id,
            route: input.route.to_string(),
            scope: input.scope.to_string(),
            scope_key: input.scope_key.to_string(),
            provider: input.provider.to_string(),
            provider_version: input.provider_version.map(str::to_string),
            content_hash: input.content_hash.map(str::to_string),
            run_id: input.run_id,
            diagnostics_json: input.diagnostics_json,
        }
    }
}
