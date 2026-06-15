use crate::RouteObservationInput;

use serde_json::Value;

#[derive(Debug, Clone)]
pub(crate) struct OwnedRouteObservationInput {
    pub(crate) workspace_id: i64,
    pub(crate) run_id: i64,
    pub(crate) route: String,
    pub(crate) scope: String,
    pub(crate) scope_key: String,
    pub(crate) provider: String,
    pub(crate) entity_kind: String,
    pub(crate) entity_id: String,
    pub(crate) source_file_id: Option<i64>,
    pub(crate) properties_json: Value,
}

impl From<RouteObservationInput<'_>> for OwnedRouteObservationInput {
    fn from(input: RouteObservationInput<'_>) -> Self {
        Self {
            workspace_id: input.workspace_id,
            run_id: input.run_id,
            route: input.route.to_string(),
            scope: input.scope.to_string(),
            scope_key: input.scope_key.to_string(),
            provider: input.provider.to_string(),
            entity_kind: input.entity_kind.to_string(),
            entity_id: input.entity_id.to_string(),
            source_file_id: input.source_file_id,
            properties_json: input.properties_json,
        }
    }
}
