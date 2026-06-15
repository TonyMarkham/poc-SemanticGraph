use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct RawDatabaseConfig {
    pub(crate) path: Option<std::path::PathBuf>,
}
