use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FtsSearchRequest {
    pub query: String,
    pub limit: Option<i64>,
    pub language: Option<String>,
    pub path_prefix: Option<String>,
    pub case_sensitive: Option<bool>,
    pub context_lines: Option<i64>,
    pub cursor: Option<String>,
}
