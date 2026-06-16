use schemars::JsonSchema;
use semantic_graph_query::ProjectionRequest;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionParams {
    pub limit: Option<i64>,
}

impl From<ProjectionParams> for ProjectionRequest {
    fn from(value: ProjectionParams) -> Self {
        Self { limit: value.limit }
    }
}
