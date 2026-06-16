use schemars::JsonSchema;
use semantic_graph_query::EdgeDetailsRequest;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EdgeDetailsParams {
    pub edge_id: String,
}

impl From<EdgeDetailsParams> for EdgeDetailsRequest {
    fn from(value: EdgeDetailsParams) -> Self {
        Self {
            edge_id: value.edge_id,
        }
    }
}
