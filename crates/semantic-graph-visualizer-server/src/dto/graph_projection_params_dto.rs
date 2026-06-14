use crate::{VisualizerServerError, VisualizerServerResult};

use serde::{Deserialize, Serialize};

const DEFAULT_LIMIT: i64 = 150;
const MAX_LIMIT: i64 = 1_000;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GraphProjectionParamsDto {
    pub limit: Option<i64>,
}

impl GraphProjectionParamsDto {
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
