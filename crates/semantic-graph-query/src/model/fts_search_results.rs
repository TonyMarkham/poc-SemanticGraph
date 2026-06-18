use crate::model::FtsSearchHit;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FtsSearchResults {
    pub requested_limit: Option<i64>,
    pub applied_limit: i64,
    pub fts_database_path: String,
    pub tantivy_index_path: String,
    pub hits: Vec<FtsSearchHit>,
    pub next_cursor: Option<String>,
}
