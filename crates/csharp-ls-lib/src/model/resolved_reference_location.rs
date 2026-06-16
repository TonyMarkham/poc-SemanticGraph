use lsp_types::Range;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedReferenceLocation {
    pub file_path: PathBuf,
    pub range: Range,
}
