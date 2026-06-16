use crate::model::{CallBatchExtraction, DocumentSymbolBatchExtraction, ReferenceBatchExtraction};

pub struct CSharpFileExtractions {
    pub file_scope_key: String,
    pub document_symbols: DocumentSymbolBatchExtraction,
    pub references: Option<ReferenceBatchExtraction>,
    pub calls: Option<CallBatchExtraction>,
}
