use lsp_types::Range;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedIncomingCall {
    pub caller_file_path: PathBuf,
    pub caller_name: String,
    pub caller_kind: String,
    pub caller_range: Range,
    pub caller_selection_range: Range,
    pub from_ranges: Vec<Range>,
}
