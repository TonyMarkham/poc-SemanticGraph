use crate::{VisualizerServerError, VisualizerServerResult};

use serde::{Deserialize, Serialize};

const DEFAULT_LIMIT: i64 = 25;
const MAX_LIMIT: i64 = 50;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GraphSearchNodesParamsDto {
    pub query: String,
    pub limit: Option<i64>,
}

impl GraphSearchNodesParamsDto {
    pub fn resolved_query(&self) -> VisualizerServerResult<String> {
        let query = self.query.trim();

        if query.is_empty() {
            return Err(VisualizerServerError::invalid_params(
                "query must not be blank".to_string(),
            ));
        }

        Ok(query.to_string())
    }

    pub fn resolved_limit(&self) -> VisualizerServerResult<i64> {
        let limit = self.limit.unwrap_or(DEFAULT_LIMIT);

        if !(1..=MAX_LIMIT).contains(&limit) {
            return Err(VisualizerServerError::invalid_params(format!(
                "limit must be between 1 and {MAX_LIMIT}"
            )));
        }

        Ok(limit)
    }
}
