#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceRouteSummary {
    pub targets_queried: usize,
    pub reference_edges: usize,
    pub reference_occurrences: usize,
    pub file_fallbacks: usize,
    pub skipped_external: usize,
}
