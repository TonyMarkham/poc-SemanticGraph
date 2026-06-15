use crate::{DbWriteProgressKind, WriteSummary};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WriteProgress {
    pub kind: DbWriteProgressKind,
    pub commands_queued: u64,
    pub commands_written: u64,
    pub commits: u64,
    pub rollbacks: u64,
}

impl WriteProgress {
    pub fn new(kind: DbWriteProgressKind, summary: WriteSummary) -> Self {
        Self {
            kind,
            commands_queued: summary.commands_queued,
            commands_written: summary.commands_written,
            commits: summary.commits,
            rollbacks: summary.rollbacks,
        }
    }
}
