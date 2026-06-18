use schemars::JsonSchema;
use semantic_graph_query::FtsSearchRequest;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FtsSearchParams {
    pub query: String,
    pub limit: Option<i64>,
    pub language: Option<String>,
    pub path_prefix: Option<String>,
    pub case_sensitive: Option<bool>,
    pub context_lines: Option<i64>,
    pub cursor: Option<String>,
}

impl From<FtsSearchParams> for FtsSearchRequest {
    fn from(value: FtsSearchParams) -> Self {
        Self {
            query: value.query,
            limit: value.limit,
            language: value.language,
            path_prefix: value.path_prefix,
            case_sensitive: value.case_sensitive,
            context_lines: value.context_lines,
            cursor: value.cursor,
        }
    }
}
