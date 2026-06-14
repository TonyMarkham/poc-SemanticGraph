use lsp_types::Range;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedOutgoingCall {
    pub target_file_path: PathBuf,
    pub target_range: Range,
    pub target_selection_range: Range,
    pub target_name: String,
    pub target_kind: String,
    pub callsite_ranges: Vec<Range>,
}
