use crate::RouteStatusFailInput;

use serde_json::Value;

#[derive(Debug, Clone)]
pub(crate) struct OwnedRouteStatusFailInput {
    pub(crate) workspace_id: i64,
    pub(crate) route: String,
    pub(crate) scope: String,
    pub(crate) scope_key: String,
    pub(crate) provider: String,
    pub(crate) run_id: i64,
    pub(crate) diagnostics_json: Value,
}

impl From<RouteStatusFailInput<'_>> for OwnedRouteStatusFailInput {
    fn from(input: RouteStatusFailInput<'_>) -> Self {
        Self {
            workspace_id: input.workspace_id,
            route: input.route.to_string(),
            scope: input.scope.to_string(),
            scope_key: input.scope_key.to_string(),
            provider: input.provider.to_string(),
            run_id: input.run_id,
            diagnostics_json: input.diagnostics_json,
        }
    }
}
