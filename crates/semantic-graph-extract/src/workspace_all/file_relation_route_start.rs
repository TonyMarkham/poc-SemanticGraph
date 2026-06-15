use crate::{model::DocumentSymbolBatchExtraction, providers::rust_analyzer::RustAnalyzerProvider};

use semantic_graph_db_manager::WriteHandle;

pub(crate) struct FileRelationRouteStart<'a> {
    pub(crate) store: &'a WriteHandle,
    pub(crate) workspace_id: i64,
    pub(crate) workspace_root_uri: &'a str,
    pub(crate) provider: &'a RustAnalyzerProvider,
    pub(crate) document_symbols: &'a DocumentSymbolBatchExtraction,
    pub(crate) workspace_fingerprint: &'a str,
    pub(crate) analysis_workers: usize,
    pub(crate) file_work_items: usize,
}
