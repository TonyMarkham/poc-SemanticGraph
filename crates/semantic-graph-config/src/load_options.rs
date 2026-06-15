use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct LoadOptions {
    pub explicit_database_path: Option<PathBuf>,
    pub explicit_config_path: Option<PathBuf>,
    pub discovery_start_dir: Option<PathBuf>,
    pub default_database_path: Option<PathBuf>,
}
