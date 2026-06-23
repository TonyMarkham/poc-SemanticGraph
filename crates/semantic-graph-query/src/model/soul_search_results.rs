use crate::model::SoulSearchResult;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SoulSearchResults {
    pub results: Vec<SoulSearchResult>,
    pub requested_limit: Option<i64>,
    pub applied_limit: i64,
    pub total_results: i64,
    pub next_cursor: Option<String>,
}
