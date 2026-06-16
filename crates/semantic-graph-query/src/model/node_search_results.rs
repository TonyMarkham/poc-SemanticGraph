use crate::model::NodeSearchResult;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NodeSearchResults {
    pub results: Vec<NodeSearchResult>,
    pub requested_limit: Option<i64>,
    pub applied_limit: i64,
}
