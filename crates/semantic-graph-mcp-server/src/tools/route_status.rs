use schemars::JsonSchema;
use semantic_graph_query::RouteStatusRequest;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RouteStatusParams {
    pub workspace_id: Option<i64>,
    pub root_uri: Option<String>,
    pub route: Option<String>,
    pub scope: Option<String>,
    pub scope_key: Option<String>,
    pub file_path: Option<String>,
    pub limit: Option<i64>,
}

impl From<RouteStatusParams> for RouteStatusRequest {
    fn from(value: RouteStatusParams) -> Self {
        Self {
            workspace_id: value.workspace_id,
            root_uri: value.root_uri,
            route: value.route,
            scope: value.scope,
            scope_key: value.scope_key,
            file_path: value.file_path,
            limit: value.limit,
        }
    }
}
