use crate::{
    ExtractError, ExtractResult,
    document_symbols::paths::file_symbol_key,
    model::{DocumentSymbolExtraction, ExtractedSymbol, GraphLanguage, SourceFile},
    providers::rust_analyzer::RustAnalyzerProvider,
};

use semantic_graph_db_manager::{ActiveFileSymbols, TextRange, WriteHandle};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

pub(crate) async fn load_unchanged_document_symbol_extractions(
    store: &WriteHandle,
    workspace_id: i64,
    provider: &RustAnalyzerProvider,
    file_uris: &HashSet<String>,
) -> ExtractResult<Vec<DocumentSymbolExtraction>> {
    if file_uris.is_empty() {
        return Ok(Vec::new());
    }

    let mut file_uris = file_uris.iter().cloned().collect::<Vec<_>>();
    file_uris.sort();
    let active_files = store
        .active_file_symbols(workspace_id, &file_uris)
        .await
        .map_err(ExtractError::storage)?;
    let mut extractions = Vec::with_capacity(active_files.len());
    for active_file in active_files {
        extractions.push(document_symbol_extraction_from_active_file(
            provider,
            active_file,
        )?);
    }

    Ok(extractions)
}

fn document_symbol_extraction_from_active_file(
    provider: &RustAnalyzerProvider,
    active_file: ActiveFileSymbols,
) -> ExtractResult<DocumentSymbolExtraction> {
    let language = graph_language_from_store(
        provider.provider_id().as_str(),
        "load active document symbols",
        &active_file.language,
    )?;
    let file_properties: Value = serde_json::from_str(&active_file.properties_json)
        .map_err(|source| ExtractError::json("parse active file properties_json", source))?;
    let raw_metadata = file_properties
        .get("raw_metadata")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let source_file = SourceFile {
        uri: active_file.uri.clone(),
        relative_path: active_file.relative_path,
        language,
        file_symbol_key: file_symbol_key(&active_file.uri),
        content_hash: active_file.content_hash,
    };
    let node_symbol_keys = active_file
        .symbols
        .iter()
        .map(|symbol| (symbol.node_id.clone(), symbol.symbol_key.clone()))
        .collect::<HashMap<_, _>>();
    let mut symbols = Vec::with_capacity(active_file.symbols.len());

    for active_symbol in active_file.symbols {
        let range = active_symbol.range.ok_or_else(|| {
            ExtractError::response_shape(
                provider.provider_id().as_str(),
                "load active document symbols",
                format!(
                    "active symbol {} is missing its range",
                    active_symbol.symbol_key
                ),
            )
        })?;
        let properties: Value = serde_json::from_str(&active_symbol.properties_json)
            .map_err(|source| ExtractError::json("parse active symbol properties_json", source))?;
        let raw_json = properties.get("raw").cloned().ok_or_else(|| {
            ExtractError::response_shape(
                provider.provider_id().as_str(),
                "load active document symbols",
                format!(
                    "active symbol {} is missing raw document-symbol metadata",
                    active_symbol.symbol_key
                ),
            )
        })?;
        let selection_range = active_symbol
            .selection_range
            .map(Ok)
            .unwrap_or_else(|| selection_range_from_symbol_raw(provider, &raw_json))?;
        let detail = properties
            .get("detail")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let parent_symbol_key = active_symbol
            .container_node_id
            .as_ref()
            .and_then(|node_id| node_symbol_keys.get(node_id))
            .cloned();

        symbols.push(ExtractedSymbol {
            provider: provider.provider_id(),
            language,
            file_uri: active_file.uri.clone(),
            symbol_key: active_symbol.symbol_key,
            parent_symbol_key,
            name: active_symbol.name,
            kind: active_symbol.kind,
            qualified_name: active_symbol.qualified_name,
            detail,
            range,
            selection_range,
            raw_json,
        });
    }

    Ok(DocumentSymbolExtraction {
        provider: provider.provider_id(),
        provider_version: rust_analyzer_lib::provider_version(),
        source_file,
        symbols,
        relations: Vec::new(),
        raw_metadata,
    })
}

fn selection_range_from_symbol_raw(
    provider: &RustAnalyzerProvider,
    raw_json: &Value,
) -> ExtractResult<TextRange> {
    let range = raw_json
        .get("document_symbol")
        .and_then(|document_symbol| document_symbol.get("selectionRange"))
        .or_else(|| raw_json.get("selectionRange"))
        .ok_or_else(|| {
            ExtractError::response_shape(
                provider.provider_id().as_str(),
                "load active document symbols",
                "active symbol raw metadata is missing selectionRange",
            )
        })?;

    text_range_from_lsp_json(provider, range)
}

fn text_range_from_lsp_json(
    provider: &RustAnalyzerProvider,
    range: &Value,
) -> ExtractResult<TextRange> {
    let start = range.get("start").ok_or_else(|| {
        ExtractError::response_shape(
            provider.provider_id().as_str(),
            "load active document symbols",
            "LSP range is missing start",
        )
    })?;
    let end = range.get("end").ok_or_else(|| {
        ExtractError::response_shape(
            provider.provider_id().as_str(),
            "load active document symbols",
            "LSP range is missing end",
        )
    })?;

    Ok(TextRange {
        start_line: json_i64(provider, start, "line")?,
        start_col: json_i64(provider, start, "character")?,
        end_line: json_i64(provider, end, "line")?,
        end_col: json_i64(provider, end, "character")?,
    })
}

fn json_i64(provider: &RustAnalyzerProvider, value: &Value, field: &str) -> ExtractResult<i64> {
    value.get(field).and_then(Value::as_i64).ok_or_else(|| {
        ExtractError::response_shape(
            provider.provider_id().as_str(),
            "load active document symbols",
            format!("LSP position is missing numeric {field}"),
        )
    })
}

fn graph_language_from_store(
    provider: &str,
    method: &str,
    language: &str,
) -> ExtractResult<GraphLanguage> {
    match language {
        "rust" => Ok(GraphLanguage::Rust),
        "csharp" => Ok(GraphLanguage::CSharp),
        _ => Err(ExtractError::response_shape(
            provider,
            method,
            format!("unsupported graph language {language}"),
        )),
    }
}
