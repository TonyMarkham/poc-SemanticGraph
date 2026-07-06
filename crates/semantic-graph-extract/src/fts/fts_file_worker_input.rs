use crate::fts::FtsDiscoveredFile;

use std::collections::HashMap;

pub(crate) struct FtsFileWorkerInput {
    pub(crate) files: Vec<FtsDiscoveredFile>,
    pub(crate) active_fts_document_hashes: HashMap<String, String>,
    pub(crate) workspace_id: i64,
    pub(crate) run_id: i64,
    pub(crate) analysis_workers: usize,
    pub(crate) max_indexed_file_bytes: u64,
    pub(crate) route: &'static str,
}
