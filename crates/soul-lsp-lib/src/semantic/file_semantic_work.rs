use crate::model::ResolvedReferenceTarget;

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSemanticWork {
    pub file_path: PathBuf,
    pub reference_targets: Vec<ResolvedReferenceTarget>,
}
