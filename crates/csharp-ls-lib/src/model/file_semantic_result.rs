use crate::model::{ResolvedIncomingCallSet, ResolvedReferenceSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSemanticResult {
    pub reference_sets: Vec<ResolvedReferenceSet>,
    pub incoming_call_sets: Vec<ResolvedIncomingCallSet>,
}
