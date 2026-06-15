use crate::{ConfigError, ConfigResult, RawWriterConfig};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriterConfig {
    queue_capacity: usize,
    max_rows_per_commit: usize,
    max_millis_per_commit: u64,
    busy_timeout_ms: u64,
}

impl WriterConfig {
    pub fn new(
        queue_capacity: usize,
        max_rows_per_commit: usize,
        max_millis_per_commit: u64,
        busy_timeout_ms: u64,
    ) -> ConfigResult<Self> {
        validate_positive_usize("writer.queue_capacity", queue_capacity)?;
        validate_positive_usize("writer.max_rows_per_commit", max_rows_per_commit)?;
        validate_positive_u64("writer.max_millis_per_commit", max_millis_per_commit)?;
        validate_positive_u64("writer.busy_timeout_ms", busy_timeout_ms)?;

        Ok(Self {
            queue_capacity,
            max_rows_per_commit,
            max_millis_per_commit,
            busy_timeout_ms,
        })
    }

    pub(crate) fn from_raw(raw: Option<RawWriterConfig>) -> ConfigResult<Self> {
        let Some(raw) = raw else {
            return Ok(Self::default());
        };

        Self::new(
            raw.queue_capacity.unwrap_or(DEFAULT_QUEUE_CAPACITY),
            raw.max_rows_per_commit
                .unwrap_or(DEFAULT_MAX_ROWS_PER_COMMIT),
            raw.max_millis_per_commit
                .unwrap_or(DEFAULT_MAX_MILLIS_PER_COMMIT),
            raw.busy_timeout_ms.unwrap_or(DEFAULT_BUSY_TIMEOUT_MS),
        )
    }

    pub fn queue_capacity(&self) -> usize {
        self.queue_capacity
    }

    pub fn max_rows_per_commit(&self) -> usize {
        self.max_rows_per_commit
    }

    pub fn max_millis_per_commit(&self) -> u64 {
        self.max_millis_per_commit
    }

    pub fn busy_timeout_ms(&self) -> u64 {
        self.busy_timeout_ms
    }
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self {
            queue_capacity: DEFAULT_QUEUE_CAPACITY,
            max_rows_per_commit: DEFAULT_MAX_ROWS_PER_COMMIT,
            max_millis_per_commit: DEFAULT_MAX_MILLIS_PER_COMMIT,
            busy_timeout_ms: DEFAULT_BUSY_TIMEOUT_MS,
        }
    }
}

const DEFAULT_QUEUE_CAPACITY: usize = 4096;
const DEFAULT_MAX_ROWS_PER_COMMIT: usize = 1000;
const DEFAULT_MAX_MILLIS_PER_COMMIT: u64 = 250;
const DEFAULT_BUSY_TIMEOUT_MS: u64 = 5000;

fn validate_positive_usize(setting: &str, value: usize) -> ConfigResult<()> {
    if value == 0 {
        return Err(ConfigError::invalid_writer_setting(
            setting,
            "must be greater than zero",
        ));
    }

    Ok(())
}

fn validate_positive_u64(setting: &str, value: u64) -> ConfigResult<()> {
    if value == 0 {
        return Err(ConfigError::invalid_writer_setting(
            setting,
            "must be greater than zero",
        ));
    }

    Ok(())
}
