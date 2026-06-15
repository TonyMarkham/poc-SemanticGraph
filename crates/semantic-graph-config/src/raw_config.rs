use crate::{
    Config, ConfigError, ConfigResult, DatabaseConfig, ExtractorConfig, RawDatabaseConfig,
    RawExtractorConfig, RawWriterConfig, WriterConfig,
};

use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Debug, Deserialize)]
struct RawConfig {
    database: Option<RawDatabaseConfig>,
    extractor: Option<RawExtractorConfig>,
    writer: Option<RawWriterConfig>,
}

pub fn load_config(path: impl AsRef<Path>) -> ConfigResult<Config> {
    let path = path.as_ref();
    let contents = fs::read_to_string(path).map_err(|source| {
        ConfigError::io(
            "read refactor radar config",
            Some(path.to_path_buf()),
            source,
        )
    })?;
    let raw =
        toml::from_str::<RawConfig>(&contents).map_err(|source| ConfigError::toml(path, source))?;
    let database_path = raw
        .database
        .and_then(|database| database.path)
        .ok_or_else(|| ConfigError::missing_database_path(Some(path.to_path_buf())))?;

    let extractor = ExtractorConfig::from_raw(raw.extractor)?;
    let writer = WriterConfig::from_raw(raw.writer)?;

    Ok(Config::new(
        DatabaseConfig::new(database_path),
        extractor,
        writer,
    ))
}
