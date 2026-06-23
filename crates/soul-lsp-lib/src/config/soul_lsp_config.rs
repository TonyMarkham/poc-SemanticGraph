use crate::config::{SoulLspPluginConfig, SoulLspScanConfig};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SoulLspConfig {
    scan: SoulLspScanConfig,
    plugins: Vec<SoulLspPluginConfig>,
}

impl SoulLspConfig {
    pub fn new(scan: SoulLspScanConfig, plugins: Vec<SoulLspPluginConfig>) -> Self {
        Self { scan, plugins }
    }

    pub fn scan(&self) -> &SoulLspScanConfig {
        &self.scan
    }

    pub fn plugins(&self) -> &[SoulLspPluginConfig] {
        &self.plugins
    }
}
