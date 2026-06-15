use crate::{
    benchmark::BenchmarkSummary,
    model::{CallRouteSummary, ReferenceRouteSummary},
    persist::PersistenceSummary,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceAllSummary {
    pub benchmark: BenchmarkSummary,
    pub document_summary: PersistenceSummary,
    pub reference_summary: PersistenceSummary,
    pub call_summary: PersistenceSummary,
    pub reference_route_summary: ReferenceRouteSummary,
    pub call_route_summary: CallRouteSummary,
}
