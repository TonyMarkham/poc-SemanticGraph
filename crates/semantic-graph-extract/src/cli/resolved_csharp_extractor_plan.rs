use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCSharpExtractorPlan {
    binary: PathBuf,
    solution: PathBuf,
    log_level: String,
    features: Vec<String>,
    process_workers: usize,
    startup_timeout_ms: u64,
    request_timeout_ms: u64,
}

impl ResolvedCSharpExtractorPlan {
    pub fn new(
        binary: PathBuf,
        solution: PathBuf,
        log_level: String,
        features: Vec<String>,
        process_workers: usize,
        startup_timeout_ms: u64,
        request_timeout_ms: u64,
    ) -> Self {
        Self {
            binary,
            solution,
            log_level,
            features,
            process_workers,
            startup_timeout_ms,
            request_timeout_ms,
        }
    }

    pub fn binary(&self) -> &PathBuf {
        &self.binary
    }

    pub fn solution(&self) -> &PathBuf {
        &self.solution
    }

    pub fn log_level(&self) -> &str {
        &self.log_level
    }

    pub fn features(&self) -> &[String] {
        &self.features
    }

    pub fn process_workers(&self) -> usize {
        self.process_workers
    }

    pub fn startup_timeout_ms(&self) -> u64 {
        self.startup_timeout_ms
    }

    pub fn request_timeout_ms(&self) -> u64 {
        self.request_timeout_ms
    }
}
