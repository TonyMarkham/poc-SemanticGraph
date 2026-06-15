use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Stopwatch {
    started_at: Instant,
}

impl Stopwatch {
    pub fn start_new() -> Self {
        Self {
            started_at: Instant::now(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }
}
