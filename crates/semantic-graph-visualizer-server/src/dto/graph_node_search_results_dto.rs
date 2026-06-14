use crate::dto::GraphNodeSearchResultDto;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodeSearchResultsDto {
    pub results: Vec<GraphNodeSearchResultDto>,
}
