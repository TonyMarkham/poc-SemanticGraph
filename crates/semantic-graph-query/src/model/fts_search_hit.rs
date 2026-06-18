use crate::model::{FtsLineRange, FtsSearchSnippet};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FtsSearchHit {
    pub uri: String,
    pub path: String,
    pub language: String,
    pub content_hash: String,
    pub score: f32,
    pub line_range: FtsLineRange,
    pub snippets: Vec<FtsSearchSnippet>,
}
