use lsp_types::Range;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCallTarget {
    pub file_path: PathBuf,
    pub selection_range: Range,
    pub name: String,
}
