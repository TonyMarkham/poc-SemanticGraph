use std::fs;

use lsp_types::{DocumentSymbol, DocumentSymbolResponse};
use serde_json::{Value, json};

use crate::document_symbols::mapper::{
    build_symbol_key, lsp_symbol_kind_name, normalize_symbol_kind, qualified_name,
    text_range_from_lsp,
};
use crate::document_symbols::paths::{
    content_hash, file_symbol_key, file_uri, validate_document_symbol_request,
    workspace_relative_path,
};
use crate::error::{ExtractError, Result};
use crate::model::{
    DocumentSymbolExtraction, DocumentSymbolRequest, ExtractedRelation, ExtractedSymbol,
    GraphLanguage, ProviderId, SourceFile,
};

pub struct RustDocumentSymbolMapper;

impl RustDocumentSymbolMapper {
    pub fn map_response(
        request: DocumentSymbolRequest,
        response: DocumentSymbolResponse,
        provider_version: Option<String>,
        raw_metadata: Value,
    ) -> Result<DocumentSymbolExtraction> {
        match response {
            DocumentSymbolResponse::Nested(symbols) => {
                if symbols.is_empty() {
                    return Err(ExtractError::response_shape(
                        ProviderId::rust_analyzer().as_str(),
                        "textDocument/documentSymbol",
                        "rust-analyzer returned an empty document symbol result",
                    ));
                }

                Self::map_nested_symbols(request, &symbols, provider_version, raw_metadata)
            }
            DocumentSymbolResponse::Flat(_) => Err(ExtractError::response_shape(
                ProviderId::rust_analyzer().as_str(),
                "textDocument/documentSymbol",
                "expected hierarchical DocumentSymbol[] but received SymbolInformation[]",
            )),
        }
    }

    pub fn map_nested_symbols(
        request: DocumentSymbolRequest,
        symbols: &[DocumentSymbol],
        provider_version: Option<String>,
        raw_metadata: Value,
    ) -> Result<DocumentSymbolExtraction> {
        let request = validate_document_symbol_request(request)?;
        let file_contents = fs::read_to_string(&request.file_path).map_err(|source| {
            ExtractError::io(
                "read source file for document symbol mapping",
                Some(request.file_path.clone()),
                source,
            )
        })?;
        let uri = file_uri(&request.file_path)?;
        let relative_path = workspace_relative_path(&request.workspace_root, &request.file_path)?;
        let source_file = SourceFile {
            file_symbol_key: file_symbol_key(&uri),
            uri: uri.clone(),
            relative_path,
            language: GraphLanguage::Rust,
            content_hash: Some(content_hash(&file_contents)),
        };

        let mut extraction = DocumentSymbolExtraction {
            provider: ProviderId::rust_analyzer(),
            provider_version,
            source_file,
            symbols: Vec::new(),
            relations: Vec::new(),
            raw_metadata,
        };

        for symbol in symbols {
            Self::map_symbol(symbol, None, &[], &mut extraction)?;
        }

        Ok(extraction)
    }

    fn map_symbol(
        symbol: &DocumentSymbol,
        parent_symbol_key: Option<&str>,
        parent_path: &[String],
        extraction: &mut DocumentSymbolExtraction,
    ) -> Result<()> {
        let kind = normalize_symbol_kind(symbol.kind).to_string();
        let selection_range = text_range_from_lsp(symbol.selection_range);
        let symbol_key = build_symbol_key(
            &extraction.source_file.uri,
            &kind,
            selection_range,
            &symbol.name,
            parent_path,
        );
        let raw_json = serde_json::to_value(symbol).map_err(|source| {
            ExtractError::json("serialize rust-analyzer document symbol", source)
        })?;
        let source_symbol_key = parent_symbol_key
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| extraction.source_file.file_symbol_key.clone());

        extraction.relations.push(ExtractedRelation {
            provider: ProviderId::rust_analyzer(),
            source_symbol_key,
            target_symbol_key: symbol_key.clone(),
            relation: "contains".to_string(),
            confidence: "EXTRACTED".to_string(),
            confidence_score: 1.0,
            range: Some(text_range_from_lsp(symbol.range)),
            raw_json: json!({
                "lsp_method": "textDocument/documentSymbol",
                "symbol": raw_json.clone()
            }),
        });

        extraction.symbols.push(ExtractedSymbol {
            provider: ProviderId::rust_analyzer(),
            language: GraphLanguage::Rust,
            file_uri: extraction.source_file.uri.clone(),
            symbol_key: symbol_key.clone(),
            parent_symbol_key: parent_symbol_key.map(ToOwned::to_owned),
            name: symbol.name.clone(),
            kind,
            qualified_name: Some(qualified_name(parent_path, &symbol.name)),
            detail: symbol.detail.clone(),
            range: text_range_from_lsp(symbol.range),
            selection_range,
            raw_json: json!({
                "lsp_kind": lsp_symbol_kind_name(symbol.kind),
                "document_symbol": raw_json
            }),
        });

        let mut child_parent_path = parent_path.to_vec();
        child_parent_path.push(symbol.name.clone());
        if let Some(children) = &symbol.children {
            for child in children {
                Self::map_symbol(child, Some(&symbol_key), &child_parent_path, extraction)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::error::Error;

    use crate::model::DocumentSymbolRequest;

    use super::*;

    #[test]
    fn rejects_flat_symbol_information_response() -> std::result::Result<(), Box<dyn Error>> {
        let value = serde_json::json!([
            {
                "name": "flat",
                "kind": 12,
                "location": {
                    "uri": "file:///tmp/lib.rs",
                    "range": {
                        "start": { "line": 0, "character": 0 },
                        "end": { "line": 0, "character": 4 }
                    }
                }
            }
        ]);
        let response: DocumentSymbolResponse = serde_json::from_value(value)?;
        let cwd = env::current_dir()?;

        let result = RustDocumentSymbolMapper::map_response(
            DocumentSymbolRequest {
                workspace_root: cwd.clone(),
                package_path: cwd.join("crates/wip"),
                file_path: cwd.join("crates/wip/src/lib.rs"),
            },
            response,
            None,
            json!({}),
        );

        assert!(matches!(result, Err(ExtractError::ResponseShape { .. })));
        Ok(())
    }
}
