use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct RawWriterConfig {
    pub(crate) queue_capacity: Option<usize>,
    pub(crate) max_rows_per_commit: Option<usize>,
    pub(crate) max_millis_per_commit: Option<u64>,
    pub(crate) busy_timeout_ms: Option<u64>,
}
