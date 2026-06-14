#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallRouteSummary {
    pub callable_nodes: usize,
    pub call_edges: usize,
    pub call_occurrences: usize,
    pub skipped_external_targets: usize,
    pub skipped_unresolved_targets: usize,
    pub skipped_non_callable_prepare_items: usize,
}
