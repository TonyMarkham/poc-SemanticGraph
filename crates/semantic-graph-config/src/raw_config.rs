use crate::{
    CSharpConfig, Config, ConfigError, ConfigResult, DatabaseConfig, ExtractorConfig, FtsConfig,
    QueryServiceConfig, RawCSharpConfig, RawDatabaseConfig, RawExtractorConfig, RawFtsConfig,
    RawQueryServiceConfig, RawSoulConfig, RawWriterConfig, SoulConfig, WriterConfig,
};

use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Debug, Deserialize)]
struct RawConfig {
    database: Option<RawDatabaseConfig>,
    extractor: Option<RawExtractorConfig>,
    writer: Option<RawWriterConfig>,
    #[serde(rename = "query-service")]
    query_service: Option<RawQueryServiceConfig>,
    csharp: Option<RawCSharpConfig>,
    soul: Option<RawSoulConfig>,
    fts: Option<RawFtsConfig>,
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
    let query_service = QueryServiceConfig::from_raw(raw.query_service)?;
    let csharp = CSharpConfig::from_raw(raw.csharp)?;
    let soul = SoulConfig::from_raw(raw.soul)?;
    let fts = FtsConfig::from_raw(raw.fts)?;

    Ok(Config::new(
        DatabaseConfig::new(database_path),
        extractor,
        writer,
        query_service,
        csharp,
        soul,
        fts,
    ))
}
