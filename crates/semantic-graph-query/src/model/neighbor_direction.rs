use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NeighborDirection {
    Incoming,
    Outgoing,
    Both,
}

impl NeighborDirection {
    pub fn includes_incoming(self) -> bool {
        matches!(self, Self::Incoming | Self::Both)
    }

    pub fn includes_outgoing(self) -> bool {
        matches!(self, Self::Outgoing | Self::Both)
    }
}
