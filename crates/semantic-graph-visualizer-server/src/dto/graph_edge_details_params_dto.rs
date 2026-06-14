use crate::{VisualizerServerError, VisualizerServerResult};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdgeDetailsParamsDto {
    pub edge_id: String,
}

impl GraphEdgeDetailsParamsDto {
    pub fn resolved_edge_id(&self) -> VisualizerServerResult<String> {
        let edge_id = self.edge_id.trim();

        if edge_id.is_empty() {
            return Err(VisualizerServerError::invalid_params(
                "edgeId must not be blank".to_string(),
            ));
        }

        Ok(edge_id.to_string())
    }
}
