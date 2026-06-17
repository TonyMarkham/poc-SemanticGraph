use crate::{ConfigError, ConfigResult, RawFtsConfig};

use std::{collections::BTreeSet, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FtsConfig {
    db_path: Option<PathBuf>,
    analysis_workers: Option<usize>,
    max_indexed_file_bytes: u64,
    ignore_directories: Vec<String>,
    ignore_files: Vec<String>,
}

impl FtsConfig {
    pub fn new(ignore_directories: Vec<String>, ignore_files: Vec<String>) -> ConfigResult<Self> {
        Self::with_values(
            None,
            None,
            DEFAULT_MAX_INDEXED_FILE_BYTES,
            ignore_directories,
            ignore_files,
        )
    }

    pub fn with_values(
        db_path: Option<PathBuf>,
        analysis_workers: Option<usize>,
        max_indexed_file_bytes: u64,
        ignore_directories: Vec<String>,
        ignore_files: Vec<String>,
    ) -> ConfigResult<Self> {
        validate_positive_optional_usize("fts.analysis_workers", analysis_workers)?;
        validate_positive_u64("fts.max_indexed_file_bytes", max_indexed_file_bytes)?;

        Ok(Self {
            db_path,
            analysis_workers,
            max_indexed_file_bytes,
            ignore_directories: normalize_paths("fts.ignore-directories", ignore_directories)?,
            ignore_files: normalize_paths("fts.ignore-files", ignore_files)?,
        })
    }

    pub(crate) fn from_raw(raw: Option<RawFtsConfig>) -> ConfigResult<Self> {
        let Some(raw) = raw else {
            return Ok(Self::default());
        };

        Self::with_values(
            raw.db_path,
            raw.analysis_workers,
            raw.max_indexed_file_bytes
                .unwrap_or(DEFAULT_MAX_INDEXED_FILE_BYTES),
            raw.ignore_directories.unwrap_or_default(),
            raw.ignore_files.unwrap_or_default(),
        )
    }

    pub fn db_path(&self) -> Option<&PathBuf> {
        self.db_path.as_ref()
    }

    pub fn analysis_workers(&self) -> Option<usize> {
        self.analysis_workers
    }

    pub fn max_indexed_file_bytes(&self) -> u64 {
        self.max_indexed_file_bytes
    }

    pub fn ignore_directories(&self) -> &[String] {
        &self.ignore_directories
    }

    pub fn ignore_files(&self) -> &[String] {
        &self.ignore_files
    }
}

impl Default for FtsConfig {
    fn default() -> Self {
        Self {
            db_path: None,
            analysis_workers: None,
            max_indexed_file_bytes: DEFAULT_MAX_INDEXED_FILE_BYTES,
            ignore_directories: Vec::new(),
            ignore_files: Vec::new(),
        }
    }
}

pub const DEFAULT_MAX_INDEXED_FILE_BYTES: u64 = 209_715_200;

fn validate_positive_optional_usize(setting: &str, value: Option<usize>) -> ConfigResult<()> {
    if value == Some(0) {
        return Err(ConfigError::invalid_fts_setting(
            setting,
            "must be greater than zero",
        ));
    }

    Ok(())
}

fn validate_positive_u64(setting: &str, value: u64) -> ConfigResult<()> {
    if value == 0 {
        return Err(ConfigError::invalid_fts_setting(
            setting,
            "must be greater than zero",
        ));
    }

    Ok(())
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
