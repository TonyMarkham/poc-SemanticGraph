use crate::model::{DocumentSymbolBatchExtraction, ReferenceBatchExtraction};

pub struct SoulFileExtractions {
    pub file_scope_key: String,
    pub document_symbols: DocumentSymbolBatchExtraction,
    pub references: Option<ReferenceBatchExtraction>,
}
