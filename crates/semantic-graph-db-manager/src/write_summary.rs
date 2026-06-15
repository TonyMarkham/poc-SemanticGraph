#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WriteSummary {
    pub commands_queued: u64,
    pub commands_written: u64,
    pub commits: u64,
    pub rollbacks: u64,
}
