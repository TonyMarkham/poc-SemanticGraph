use crate::{ConfigError, ConfigResult, RawSoulScanConfig};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoulScanConfig {
    excluded_dirs: Vec<String>,
    excluded_dir_suffixes: Vec<String>,
    excluded_bin_except_under: Vec<String>,
}

impl SoulScanConfig {
    pub fn new(
        excluded_dirs: Vec<String>,
        excluded_dir_suffixes: Vec<String>,
        excluded_bin_except_under: Vec<String>,
    ) -> ConfigResult<Self> {
        Ok(Self {
            excluded_dirs: normalize_values("soul.scan.excluded_dirs", excluded_dirs)?,
            excluded_dir_suffixes: normalize_values(
                "soul.scan.excluded_dir_suffixes",
                excluded_dir_suffixes,
            )?,
            excluded_bin_except_under: normalize_values(
                "soul.scan.excluded_bin_except_under",
                excluded_bin_except_under,
            )?,
        })
    }

    pub(crate) fn from_raw(raw: Option<RawSoulScanConfig>) -> ConfigResult<Self> {
        let Some(raw) = raw else {
            return Ok(Self::default());
        };

        Self::new(
            raw.excluded_dirs.unwrap_or_else(default_excluded_dirs),
            raw.excluded_dir_suffixes
                .unwrap_or_else(default_excluded_dir_suffixes),
            raw.excluded_bin_except_under
                .unwrap_or_else(default_excluded_bin_except_under),
        )
    }

    pub fn excluded_dirs(&self) -> &[String] {
        &self.excluded_dirs
    }

    pub fn excluded_dir_suffixes(&self) -> &[String] {
        &self.excluded_dir_suffixes
    }

    pub fn excluded_bin_except_under(&self) -> &[String] {
        &self.excluded_bin_except_under
    }
}

impl Default for SoulScanConfig {
    fn default() -> Self {
        Self {
            excluded_dirs: default_excluded_dirs(),
            excluded_dir_suffixes: default_excluded_dir_suffixes(),
            excluded_bin_except_under: default_excluded_bin_except_under(),
        }
    }
}

fn normalize_values(setting: &str, values: Vec<String>) -> ConfigResult<Vec<String>> {
    let mut unique = Vec::new();
    for value in values {
        let normalized = value.trim().to_string();
        if normalized.is_empty() {
            return Err(ConfigError::invalid_soul_setting(
                setting,
                "value must not be empty",
            ));
        }
        if !unique.contains(&normalized) {
            unique.push(normalized);
        }
    }

    Ok(unique)
}

fn default_excluded_dirs() -> Vec<String> {
    [
        ".git",
        ".soul",
        "target",
        ".idea",
        ".vscode",
        ".vs",
        ".codex",
        "node_modules",
        "obj",
    ]
    .into_iter()
    .map(ToString::to_string)
    .collect()
}

fn default_excluded_dir_suffixes() -> Vec<String> {
    ["Tests", ".Tests", "tests", ".tests"]
        .into_iter()
        .map(ToString::to_string)
        .collect()
}

fn default_excluded_bin_except_under() -> Vec<String> {
    ["src"].into_iter().map(ToString::to_string).collect()
}
