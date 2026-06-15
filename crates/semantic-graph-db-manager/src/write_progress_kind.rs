#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbWriteProgressKind {
    ManagerStarted,
    CommandQueued,
    CommandWriting,
    CommandCommitted,
    CommandFailed,
    ManagerShutdown,
}
