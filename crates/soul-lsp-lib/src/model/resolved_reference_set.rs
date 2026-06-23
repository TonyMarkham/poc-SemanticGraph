use crate::model::ResolvedReferenceLocation;

use lsp_types::Range;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedReferenceSet {
    pub target_file_path: PathBuf,
    pub target_selection_range: Range,
    pub target_name: String,
    pub references: Vec<ResolvedReferenceLocation>,
}
