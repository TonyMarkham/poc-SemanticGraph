use schemars::JsonSchema;
use semantic_graph_query::NodeDetailsRequest;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NodeDetailsParams {
    pub node_id: String,
}

impl From<NodeDetailsParams> for NodeDetailsRequest {
    fn from(value: NodeDetailsParams) -> Self {
        Self {
            node_id: value.node_id,
        }
    }
}
