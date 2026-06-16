use crate::install::{AssetSource, InstallManifestMode, ManagedFileManifestEntry};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InstallManifest {
    pub schema_version: u32,
    pub installer_crate: String,
    pub installer_version: String,
    pub project_root: String,
    pub mode: InstallManifestMode,
    pub asset_source: AssetSource,
    pub managed_files: Vec<ManagedFileManifestEntry>,
}

impl InstallManifest {
    pub fn checksum_for_path(&self, relative_path: &Path) -> Option<&str> {
        let needle = relative_path.to_string_lossy();
        self.managed_files
            .iter()
            .find(|entry| entry.path == needle)
            .map(|entry| entry.sha256.as_str())
    }
}
