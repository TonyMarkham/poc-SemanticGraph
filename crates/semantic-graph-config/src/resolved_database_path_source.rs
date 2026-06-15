#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedDatabasePathSource {
    ExplicitDatabasePath,
    ExplicitConfig,
    DiscoveredConfig,
    Default,
}
