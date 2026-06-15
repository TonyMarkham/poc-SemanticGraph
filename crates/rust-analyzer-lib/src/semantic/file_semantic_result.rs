use crate::model::{ResolvedOutgoingCallSet, ResolvedReferenceSet};

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSemanticResult {
    pub file_path: PathBuf,
    pub reference_sets: Vec<ResolvedReferenceSet>,
    pub call_sets: Vec<ResolvedOutgoingCallSet>,
}
