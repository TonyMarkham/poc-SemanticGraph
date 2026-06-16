use schemars::JsonSchema;
use semantic_graph_query::{NeighborDirection, NeighborsRequest};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NeighborsParams {
    pub node_id: String,
    pub direction: Option<NeighborDirectionParam>,
    pub relation: Option<String>,
    pub limit: Option<i64>,
}

impl From<NeighborsParams> for NeighborsRequest {
    fn from(value: NeighborsParams) -> Self {
        Self {
            node_id: value.node_id,
            direction: value
                .direction
                .map(NeighborDirectionParam::into_query_direction),
            relation: value.relation,
            limit: value.limit,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NeighborDirectionParam {
    Incoming,
    Outgoing,
    Both,
}

impl NeighborDirectionParam {
    fn into_query_direction(self) -> NeighborDirection {
        match self {
            Self::Incoming => NeighborDirection::Incoming,
            Self::Outgoing => NeighborDirection::Outgoing,
            Self::Both => NeighborDirection::Both,
        }
    }
}
