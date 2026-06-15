use crate::model::{ResolvedCallTarget, ResolvedReferenceTarget};

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSemanticWork {
    pub file_path: PathBuf,
    pub reference_targets: Vec<ResolvedReferenceTarget>,
    pub call_targets: Vec<ResolvedCallTarget>,
}
