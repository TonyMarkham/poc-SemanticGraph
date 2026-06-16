use crate::args::McpInstallMode;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InstallManifestMode {
    pub mcp: McpInstallMode,
    pub database_path: Option<String>,
}

impl InstallManifestMode {
    pub fn new(mcp: McpInstallMode, database_path: Option<String>) -> Self {
        Self { mcp, database_path }
    }
}
