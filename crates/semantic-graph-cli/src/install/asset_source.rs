use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AssetSource {
    pub manifest_path: String,
    pub asset_generation: String,
}

impl AssetSource {
    pub fn new(manifest_path: impl Into<String>, asset_generation: impl Into<String>) -> Self {
        Self {
            manifest_path: manifest_path.into(),
            asset_generation: asset_generation.into(),
        }
    }
}
