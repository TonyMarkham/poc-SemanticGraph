use crate::model::{DocumentSymbolBatchExtraction, DocumentSymbolExtraction, ProviderId};

use serde_json::json;

pub fn combined_document_symbols(
    provider: ProviderId,
    facade: &str,
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
        provider,
        provider_version,
        extractions,
        raw_metadata: json!({
            "facade": facade,
            "incremental": true,
        }),
    }
}
