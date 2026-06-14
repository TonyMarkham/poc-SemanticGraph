use crate::ResolvedOutgoingCall;

use lsp_types::Range;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedOutgoingCallSet {
    pub caller_file_path: PathBuf,
    pub caller_selection_range: Range,
    pub caller_name: String,
    pub outgoing_calls: Vec<ResolvedOutgoingCall>,
    pub skipped_non_callable_prepare_items: usize,
}
