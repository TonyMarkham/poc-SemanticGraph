use crate::fts::FtsSkipReason;

use semantic_graph_db_manager::{FtsWriteBatchDocumentInput, FtsWriteBatchSeenDocumentInput};
use std::time::Duration;

#[derive(Debug, Clone)]
pub(crate) struct FtsFileWorkResult {
    pub(crate) document: Option<FtsWriteBatchDocumentInput>,
    pub(crate) seen_document: Option<FtsWriteBatchSeenDocumentInput>,
    pub(crate) fingerprint_entry: Option<String>,
    pub(crate) skip_reason: Option<FtsSkipReason>,
    pub(crate) indexed_bytes: usize,
    pub(crate) file_read_elapsed: Duration,
    pub(crate) file_hash_elapsed: Duration,
    pub(crate) file_uri_elapsed: Duration,
}

impl FtsFileWorkResult {
    pub(crate) fn skipped(reason: FtsSkipReason, file_read_elapsed: Duration) -> Self {
        Self {
            document: None,
            seen_document: None,
            fingerprint_entry: None,
            skip_reason: Some(reason),
            indexed_bytes: 0,
            file_read_elapsed,
            file_hash_elapsed: Duration::from_millis(0),
            file_uri_elapsed: Duration::from_millis(0),
        }
    }
}
