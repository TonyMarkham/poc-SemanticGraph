#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    queue_capacity: usize,
    max_rows_per_commit: usize,
    max_millis_per_commit: u64,
    busy_timeout_ms: u64,
}

impl From<&semantic_graph_config::WriterConfig> for Config {
    fn from(config: &semantic_graph_config::WriterConfig) -> Self {
        Self::new(
            config.queue_capacity(),
            config.max_rows_per_commit(),
            config.max_millis_per_commit(),
            config.busy_timeout_ms(),
        )
    }
}

impl Config {
    pub fn new(
        queue_capacity: usize,
        max_rows_per_commit: usize,
        max_millis_per_commit: u64,
        busy_timeout_ms: u64,
    ) -> Self {
        Self {
            queue_capacity,
            max_rows_per_commit,
            max_millis_per_commit,
            busy_timeout_ms,
        }
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

impl Default for Config {
    fn default() -> Self {
        Self {
            queue_capacity: 4096,
            max_rows_per_commit: 1000,
            max_millis_per_commit: 250,
            busy_timeout_ms: 5000,
        }
    }
}
