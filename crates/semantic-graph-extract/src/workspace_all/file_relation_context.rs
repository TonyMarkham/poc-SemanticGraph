use crate::{
    model::{DocumentSymbolBatchExtraction, DocumentSymbolBatchRequest},
    providers::rust_analyzer::RustAnalyzerProvider,
};

use semantic_graph_db_manager::WriteHandle;
use std::{collections::HashMap, sync::Arc};

#[derive(Clone)]
pub(crate) struct FileRelationContext {
    pub(crate) store: WriteHandle,
    pub(crate) provider: RustAnalyzerProvider,
    pub(crate) analysis_worker: rust_analyzer_lib::AnalysisWorkerHandle,
    pub(crate) document_request: DocumentSymbolBatchRequest,
    pub(crate) document_symbols: DocumentSymbolBatchExtraction,
    pub(crate) file_ids: Arc<HashMap<String, i64>>,
    pub(crate) workspace_id: i64,
    pub(crate) workspace_root_uri: String,
    pub(crate) workspace_fingerprint: String,
    pub(crate) reference_run_id: i64,
    pub(crate) call_run_id: i64,
    pub(crate) analysis_worker_count: usize,
}
