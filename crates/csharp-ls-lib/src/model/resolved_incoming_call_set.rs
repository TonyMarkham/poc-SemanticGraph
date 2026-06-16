use crate::model::ResolvedIncomingCall;

use lsp_types::Range;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedIncomingCallSet {
    pub target_file_path: PathBuf,
    pub target_selection_range: Range,
    pub incoming_calls: Vec<ResolvedIncomingCall>,
    pub skipped_non_callable_prepare_items: usize,
}
