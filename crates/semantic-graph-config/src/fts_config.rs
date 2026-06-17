use crate::{ConfigError, ConfigResult, RawFtsConfig};

use std::collections::BTreeSet;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FtsConfig {
    ignore_directories: Vec<String>,
    ignore_files: Vec<String>,
}

impl FtsConfig {
    pub fn new(ignore_directories: Vec<String>, ignore_files: Vec<String>) -> ConfigResult<Self> {
        Ok(Self {
            ignore_directories: normalize_paths("fts.ignore-directories", ignore_directories)?,
            ignore_files: normalize_paths("fts.ignore-files", ignore_files)?,
        })
    }

    pub(crate) fn from_raw(raw: Option<RawFtsConfig>) -> ConfigResult<Self> {
        let Some(raw) = raw else {
            return Ok(Self::default());
        };

        Self::new(
            raw.ignore_directories.unwrap_or_default(),
            raw.ignore_files.unwrap_or_default(),
        )
    }

    pub fn ignore_directories(&self) -> &[String] {
        &self.ignore_directories
    }

    pub fn ignore_files(&self) -> &[String] {
        &self.ignore_files
    }
}

fn normalize_paths(setting: &str, values: Vec<String>) -> ConfigResult<Vec<String>> {
    let mut unique = BTreeSet::new();
    for value in values {
        unique.insert(normalize_path(setting, &value)?);
    }

    Ok(unique.into_iter().collect())
}

fn normalize_path(setting: &str, value: &str) -> ConfigResult<String> {
    let normalized = value.trim().replace('\\', "/");
    if normalized.is_empty() {
        return Err(ConfigError::invalid_fts_setting(
            setting,
            "path must not be empty",
        ));
    }
    if normalized.starts_with('/') || has_windows_drive_prefix(&normalized) {
        return Err(ConfigError::invalid_fts_setting(
            setting,
            "path must be workspace-relative",
        ));
    }

    let mut parts = Vec::new();
    for part in normalized.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                return Err(ConfigError::invalid_fts_setting(
                    setting,
                    "path must not contain '..'",
                ));
            }
            part => parts.push(part),
        }
    }
    if parts.is_empty() {
        return Err(ConfigError::invalid_fts_setting(
            setting,
            "path must name a file or directory",
        ));
    }

    Ok(parts.join("/"))
}

fn has_windows_drive_prefix(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 3 && bytes[1] == b':' && bytes[2] == b'/' && bytes[0].is_ascii_alphabetic()
}
