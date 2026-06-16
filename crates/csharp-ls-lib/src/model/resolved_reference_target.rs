use lsp_types::Range;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedReferenceTarget {
    pub file_path: PathBuf,
    pub selection_range: Range,
}
