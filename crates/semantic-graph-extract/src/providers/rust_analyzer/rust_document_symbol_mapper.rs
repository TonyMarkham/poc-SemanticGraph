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
use crate::error::{ExtractError, ExtractResult};
use crate::model::{
    DocumentSymbolExtraction, DocumentSymbolRequest, ExtractedRelation, ExtractedSymbol,
    GraphLanguage, ProviderId, SourceFile, SourceLanguage,
};

pub struct RustDocumentSymbolMapper;

impl RustDocumentSymbolMapper {
    pub fn map_response(
        request: DocumentSymbolRequest,
        response: DocumentSymbolResponse,
        provider_version: Option<String>,
        raw_metadata: Value,
    ) -> ExtractResult<DocumentSymbolExtraction> {
        match response {
            DocumentSymbolResponse::Nested(symbols) => {
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
    ) -> ExtractResult<DocumentSymbolExtraction> {
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
            language: SourceLanguage::Rust,
            content_hash: Some(content_hash(&file_contents)),
        };

        let mut extraction = DocumentSymbolExtraction {
            provider: ProviderId::rust_analyzer(),
            language: GraphLanguage::Rust,
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
    ) -> ExtractResult<()> {
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
