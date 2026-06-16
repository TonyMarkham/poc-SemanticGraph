use crate::{ConfigError, ConfigResult, RawCSharpConfig};

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CSharpConfig {
    binary: PathBuf,
    solution: Option<PathBuf>,
    log_level: String,
    features: Vec<String>,
    analysis_workers: usize,
    startup_timeout_ms: u64,
    request_timeout_ms: u64,
}

impl CSharpConfig {
    pub fn new(
        binary: PathBuf,
        solution: Option<PathBuf>,
        log_level: String,
        features: Vec<String>,
        analysis_workers: usize,
        startup_timeout_ms: u64,
        request_timeout_ms: u64,
    ) -> ConfigResult<Self> {
        validate_non_empty_string("csharp.binary", &binary.to_string_lossy())?;
        validate_non_empty_string("csharp.log_level", &log_level)?;
        validate_positive_usize("csharp.analysis_workers", analysis_workers)?;
        validate_positive_u64("csharp.startup_timeout_ms", startup_timeout_ms)?;
        validate_positive_u64("csharp.request_timeout_ms", request_timeout_ms)?;

        Ok(Self {
            binary,
            solution,
            log_level,
            features,
            analysis_workers,
            startup_timeout_ms,
            request_timeout_ms,
        })
    }

    pub(crate) fn from_raw(raw: Option<RawCSharpConfig>) -> ConfigResult<Self> {
        let Some(raw) = raw else {
            return Ok(Self::default());
        };

        Self::new(
            raw.binary
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(DEFAULT_BINARY)),
            raw.solution.map(PathBuf::from),
            raw.log_level
                .unwrap_or_else(|| DEFAULT_LOG_LEVEL.to_string()),
            raw.features.unwrap_or_default(),
            raw.analysis_workers.unwrap_or(DEFAULT_ANALYSIS_WORKERS),
            raw.startup_timeout_ms.unwrap_or(DEFAULT_STARTUP_TIMEOUT_MS),
            raw.request_timeout_ms.unwrap_or(DEFAULT_REQUEST_TIMEOUT_MS),
        )
    }

    pub fn binary(&self) -> &PathBuf {
        &self.binary
    }

    pub fn solution(&self) -> Option<&PathBuf> {
        self.solution.as_ref()
    }

    pub fn log_level(&self) -> &str {
        &self.log_level
    }

    pub fn features(&self) -> &[String] {
        &self.features
    }

    pub fn analysis_workers(&self) -> usize {
        self.analysis_workers
    }

    pub fn startup_timeout_ms(&self) -> u64 {
        self.startup_timeout_ms
    }

    pub fn request_timeout_ms(&self) -> u64 {
        self.request_timeout_ms
    }
}

impl Default for CSharpConfig {
    fn default() -> Self {
        Self {
            binary: PathBuf::from(DEFAULT_BINARY),
            solution: None,
            log_level: DEFAULT_LOG_LEVEL.to_string(),
            features: Vec::new(),
            analysis_workers: DEFAULT_ANALYSIS_WORKERS,
            startup_timeout_ms: DEFAULT_STARTUP_TIMEOUT_MS,
            request_timeout_ms: DEFAULT_REQUEST_TIMEOUT_MS,
        }
    }
}

const DEFAULT_BINARY: &str = "csharp-ls";
const DEFAULT_LOG_LEVEL: &str = "warning";
const DEFAULT_ANALYSIS_WORKERS: usize = 1;
const DEFAULT_STARTUP_TIMEOUT_MS: u64 = 120000;
const DEFAULT_REQUEST_TIMEOUT_MS: u64 = 30000;

fn validate_non_empty_string(setting: &str, value: &str) -> ConfigResult<()> {
    if value.trim().is_empty() {
        return Err(ConfigError::invalid_csharp_setting(
            setting,
            "must not be empty",
        ));
    }

    Ok(())
}

fn validate_positive_usize(setting: &str, value: usize) -> ConfigResult<()> {
    if value == 0 {
        return Err(ConfigError::invalid_csharp_setting(
            setting,
            "must be greater than zero",
        ));
    }

    Ok(())
}

fn validate_positive_u64(setting: &str, value: u64) -> ConfigResult<()> {
    if value == 0 {
        return Err(ConfigError::invalid_csharp_setting(
            setting,
            "must be greater than zero",
        ));
    }

    Ok(())
}
