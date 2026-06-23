use crate::{ConfigResult, RawSoulConfig, SoulPluginConfig, SoulScanConfig};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SoulConfig {
    scan: SoulScanConfig,
    plugins: Vec<SoulPluginConfig>,
}

impl SoulConfig {
    pub fn new(scan: SoulScanConfig, plugins: Vec<SoulPluginConfig>) -> Self {
        Self { scan, plugins }
    }

    pub(crate) fn from_raw(raw: Option<RawSoulConfig>) -> ConfigResult<Self> {
        let Some(raw) = raw else {
            return Ok(Self::default());
        };

        let plugins = raw
            .plugins
            .into_iter()
            .map(SoulPluginConfig::from_raw)
            .collect::<ConfigResult<Vec<_>>>()?;

        Ok(Self::new(SoulScanConfig::from_raw(raw.scan)?, plugins))
    }

    pub fn scan(&self) -> &SoulScanConfig {
        &self.scan
    }

    pub fn plugins(&self) -> &[SoulPluginConfig] {
        &self.plugins
    }
}
