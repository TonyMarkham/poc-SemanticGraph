use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpServerAsset {
    pub table_name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub enabled: bool,
    pub required: bool,
    pub startup_timeout_sec: u64,
    pub tool_timeout_sec: u64,
    pub output_path: PathBuf,
}
