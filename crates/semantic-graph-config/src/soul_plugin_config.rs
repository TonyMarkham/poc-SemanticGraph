use crate::{ConfigError, ConfigResult, RawSoulPluginConfig};

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoulPluginConfig {
    language: String,
    path: PathBuf,
}

impl SoulPluginConfig {
    pub fn new(language: String, path: PathBuf) -> ConfigResult<Self> {
        validate_non_empty("soul.plugins.language", &language)?;
        validate_non_empty("soul.plugins.path", &path.to_string_lossy())?;

        Ok(Self { language, path })
    }

    pub(crate) fn from_raw(raw: RawSoulPluginConfig) -> ConfigResult<Self> {
        let language = raw.language.ok_or_else(|| {
            ConfigError::invalid_soul_setting("soul.plugins.language", "is required")
        })?;
        let path = raw
            .path
            .ok_or_else(|| ConfigError::invalid_soul_setting("soul.plugins.path", "is required"))?;

        Self::new(language, path)
    }

    pub fn language(&self) -> &str {
        &self.language
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

fn validate_non_empty(setting: &str, value: &str) -> ConfigResult<()> {
    if value.trim().is_empty() {
        return Err(ConfigError::invalid_soul_setting(
            setting,
            "must not be empty",
        ));
    }

    Ok(())
}
