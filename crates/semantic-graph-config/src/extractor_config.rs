use crate::{ConfigError, ConfigResult, ExtractorMode, RawExtractorConfig};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractorConfig {
    mode: ExtractorMode,
    jobs: Option<usize>,
    reference_jobs: Option<usize>,
    call_jobs: Option<usize>,
    analysis_workers: Option<usize>,
    reference_analysis_workers: Option<usize>,
    call_analysis_workers: Option<usize>,
}

impl ExtractorConfig {
    pub fn new(
        mode: ExtractorMode,
        jobs: Option<usize>,
        reference_jobs: Option<usize>,
        call_jobs: Option<usize>,
        analysis_workers: Option<usize>,
        reference_analysis_workers: Option<usize>,
        call_analysis_workers: Option<usize>,
    ) -> ConfigResult<Self> {
        validate_positive_optional_usize("extractor.jobs", jobs)?;
        validate_positive_optional_usize("extractor.reference_jobs", reference_jobs)?;
        validate_positive_optional_usize("extractor.call_jobs", call_jobs)?;
        validate_positive_optional_usize("extractor.analysis_workers", analysis_workers)?;

        Ok(Self {
            mode,
            jobs,
            reference_jobs,
            call_jobs,
            analysis_workers,
            reference_analysis_workers,
            call_analysis_workers,
        })
    }

    pub(crate) fn from_raw(raw: Option<RawExtractorConfig>) -> ConfigResult<Self> {
        let Some(raw) = raw else {
            return Ok(Self::default());
        };
        let mode = match raw.mode {
            Some(mode) => ExtractorMode::parse("extractor.mode", &mode)?,
            None => DEFAULT_MODE,
        };

        Self::new(
            mode,
            raw.jobs,
            raw.reference_jobs,
            raw.call_jobs,
            raw.analysis_workers,
            raw.reference_analysis_workers,
            raw.call_analysis_workers,
        )
    }

    pub fn mode(&self) -> ExtractorMode {
        self.mode
    }

    pub fn jobs(&self) -> Option<usize> {
        self.jobs
    }

    pub fn reference_jobs(&self) -> Option<usize> {
        self.reference_jobs
    }

    pub fn call_jobs(&self) -> Option<usize> {
        self.call_jobs
    }

    pub fn analysis_workers(&self) -> Option<usize> {
        self.analysis_workers
    }

    pub fn reference_analysis_workers(&self) -> Option<usize> {
        self.reference_analysis_workers
    }

    pub fn call_analysis_workers(&self) -> Option<usize> {
        self.call_analysis_workers
    }
}

impl Default for ExtractorConfig {
    fn default() -> Self {
        Self {
            mode: DEFAULT_MODE,
            jobs: None,
            reference_jobs: None,
            call_jobs: None,
            analysis_workers: None,
            reference_analysis_workers: None,
            call_analysis_workers: None,
        }
    }
}

const DEFAULT_MODE: ExtractorMode = ExtractorMode::Serial;

fn validate_positive_optional_usize(setting: &str, value: Option<usize>) -> ConfigResult<()> {
    if value == Some(0) {
        return Err(ConfigError::invalid_extractor_setting(
            setting,
            "must be greater than zero",
        ));
    }

    Ok(())
}
