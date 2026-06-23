use crate::{RawSoulPluginConfig, RawSoulScanConfig};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct RawSoulConfig {
    pub(crate) scan: Option<RawSoulScanConfig>,
    #[serde(default)]
    pub(crate) plugins: Vec<RawSoulPluginConfig>,
}
