use crate::{
    model::{DocumentSymbolBatchExtraction, DocumentSymbolExtraction},
    providers::rust_analyzer::RustAnalyzerProvider,
};

use serde_json::json;

pub(crate) fn combined_document_symbols(
    provider: &RustAnalyzerProvider,
    changed_document_symbols: DocumentSymbolBatchExtraction,
    mut loaded_extractions: Vec<DocumentSymbolExtraction>,
) -> DocumentSymbolBatchExtraction {
    let provider_version = changed_document_symbols.provider_version;
    let mut extractions = changed_document_symbols.extractions;
    extractions.append(&mut loaded_extractions);
    extractions.sort_by(|left, right| {
        left.source_file
            .relative_path
            .cmp(&right.source_file.relative_path)
    });

    DocumentSymbolBatchExtraction {
        provider: provider.provider_id(),
        provider_version,
        extractions,
        raw_metadata: json!({
            "facade": "rust-analyzer-lib",
            "incremental": true,
        }),
    }
}
