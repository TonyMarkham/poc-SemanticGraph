use crate::{VisualizerServerError, VisualizerServerResult};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodeDetailsParamsDto {
    pub node_id: String,
}

impl GraphNodeDetailsParamsDto {
    pub fn resolved_node_id(&self) -> VisualizerServerResult<String> {
        let node_id = self.node_id.trim();

        if node_id.is_empty() {
            return Err(VisualizerServerError::invalid_params(
                "nodeId must not be blank".to_string(),
            ));
        }

        Ok(node_id.to_string())
    }
}
