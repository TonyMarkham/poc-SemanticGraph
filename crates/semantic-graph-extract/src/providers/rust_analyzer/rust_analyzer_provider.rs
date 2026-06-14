use crate::{
    ExtractError, ExtractResult,
    document_symbols::mapper::text_range_from_lsp,
    document_symbols::paths::{
        validate_document_symbol_batch_request, validate_document_symbol_request,
    },
    model::{
        CallBatchExtraction, CallBatchRequest, CallOccurrence, CallRouteSummary,
        DocumentSymbolBatchExtraction, DocumentSymbolBatchRequest, DocumentSymbolExtraction,
        DocumentSymbolRequest, ExtractedCall, ExtractedReference, ExtractedSymbol, GraphLanguage,
        ProviderId, ReferenceBatchExtraction, ReferenceBatchRequest, ReferenceOccurrence,
        ReferenceRouteSummary, RouteName, SourceFile,
    },
    provider::DocumentSymbolProvider,
    providers::rust_analyzer::RustDocumentSymbolMapper,
};

use lsp_types::DocumentSymbolResponse;
use lsp_types::{Position, Range};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustAnalyzerProvider;

impl RustAnalyzerProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn with_binary(_binary: impl Into<String>) -> Self {
        Self
    }

    pub async fn extract_document_symbol_batch(
        &self,
        request: DocumentSymbolBatchRequest,
    ) -> ExtractResult<DocumentSymbolBatchExtraction> {
        self.run_batch(request).await
    }

    pub async fn extract_rust_references(
        &self,
        request: ReferenceBatchRequest,
    ) -> ExtractResult<ReferenceBatchExtraction> {
        self.run_references(request).await
    }

    pub async fn extract_rust_calls(
        &self,
        request: CallBatchRequest,
    ) -> ExtractResult<CallBatchExtraction> {
        self.run_calls(request).await
    }

    pub fn discover_rust_source_files(
        &self,
        workspace_root: &Path,
        package_path: &Path,
    ) -> ExtractResult<Vec<PathBuf>> {
        let request = validate_document_symbol_batch_request(DocumentSymbolBatchRequest {
            workspace_root: workspace_root.to_path_buf(),
            package_path: package_path.to_path_buf(),
            file_paths: Vec::new(),
        })?;
        let model = rust_analyzer_lib::load_package(&request.workspace_root, &request.package_path)
            .map_err(|source| facade_error("rust-analyzer-lib load_package", source))?;
        let files = rust_analyzer_lib::package_source_files(&model, &request.package_path);
        if files.is_empty() {
            return Err(ExtractError::response_shape(
                self.provider_id().as_str(),
                "rust-analyzer-lib package_source_files",
                "rust-analyzer-lib returned no Rust source files under the package path",
            ));
        }

        Ok(files)
    }

    pub fn discover_rust_workspace_source_files(
        &self,
        workspace_root: &Path,
    ) -> ExtractResult<Vec<PathBuf>> {
        let request = validate_document_symbol_batch_request(DocumentSymbolBatchRequest {
            workspace_root: workspace_root.to_path_buf(),
            package_path: workspace_root.to_path_buf(),
            file_paths: Vec::new(),
        })?;
        let model = rust_analyzer_lib::load_workspace(&request.workspace_root)
            .map_err(|source| facade_error("rust-analyzer-lib load_workspace", source))?;
        let files = rust_analyzer_lib::workspace_source_files(&model);
        if files.is_empty() {
            return Err(ExtractError::response_shape(
                self.provider_id().as_str(),
                "rust-analyzer-lib workspace_source_files",
                "rust-analyzer-lib returned no Rust workspace source files",
            ));
        }

        Ok(files)
    }

    async fn run(&self, request: DocumentSymbolRequest) -> ExtractResult<DocumentSymbolExtraction> {
        let request = validate_document_symbol_request(request)?;
        let mut batch = self
            .run_batch(DocumentSymbolBatchRequest {
                workspace_root: request.workspace_root,
                package_path: request.package_path,
                file_paths: vec![request.file_path],
            })
            .await?;

        batch.extractions.pop().ok_or_else(|| {
            ExtractError::response_shape(
                self.provider_id().as_str(),
                "textDocument/documentSymbol",
                "single-file document symbol extraction returned no files",
            )
        })
    }

    async fn run_batch(
        &self,
        request: DocumentSymbolBatchRequest,
    ) -> ExtractResult<DocumentSymbolBatchExtraction> {
        let request = validate_document_symbol_batch_request(request)?;
        let provider_version = rust_analyzer_lib::provider_version();
        let document_symbols = rust_analyzer_lib::document_symbols_for_files(
            &request.workspace_root,
            &request.file_paths,
        )
        .map_err(|source| facade_error("rust-analyzer-lib document_symbols_for_files", source))?;
        let mut extractions = Vec::with_capacity(document_symbols.len());

        for (file_path, symbols) in document_symbols {
            let response = DocumentSymbolResponse::Nested(symbols.clone());
            let extraction = RustDocumentSymbolMapper::map_response(
                DocumentSymbolRequest {
                    workspace_root: request.workspace_root.clone(),
                    package_path: request.package_path.clone(),
                    file_path: file_path.clone(),
                },
                response,
                provider_version.clone(),
                json!({
                    "facade": "rust-analyzer-lib",
                    "lsp_method": "textDocument/documentSymbol",
                    "document_symbol": symbols
                }),
            )?;

            extractions.push(extraction);
        }

        Ok(DocumentSymbolBatchExtraction {
            provider: self.provider_id(),
            provider_version,
            extractions,
            raw_metadata: json!({
                "facade": "rust-analyzer-lib",
                "file_count": request.file_paths.len(),
            }),
        })
    }

    async fn run_references(
        &self,
        request: ReferenceBatchRequest,
    ) -> ExtractResult<ReferenceBatchExtraction> {
        let document_request =
            validate_document_symbol_batch_request(DocumentSymbolBatchRequest {
                workspace_root: request.workspace_root,
                package_path: request.package_path,
                file_paths: request.file_paths,
            })?;
        let document_symbols = self.run_batch(document_request.clone()).await?;
        let target_contexts = reference_target_contexts(&document_request, &document_symbols)?;
        let reference_targets = target_contexts
            .iter()
            .map(|context| context.target.clone())
            .collect::<Vec<_>>();
        let reference_sets = rust_analyzer_lib::references_for_symbols(
            &document_request.workspace_root,
            &reference_targets,
        )
        .map_err(|source| facade_error("rust-analyzer-lib references_for_symbols", source))?;
        let workspace_fingerprint = workspace_fingerprint(&document_symbols);
        let symbol_index = SymbolIndex::new(&document_request.workspace_root, &document_symbols)?;
        let mut grouped_references = BTreeMap::new();
        let mut skipped_external = 0;
        let mut file_fallbacks = 0;
        let target_by_range = target_contexts
            .into_iter()
            .map(|context| {
                (
                    target_key(&context.target.file_path, context.target.selection_range),
                    context,
                )
            })
            .collect::<HashMap<_, _>>();

        for reference_set in reference_sets {
            let Some(target_context) = target_by_range.get(&target_key(
                &reference_set.target_file_path,
                reference_set.target_selection_range,
            )) else {
                return Err(ExtractError::response_shape(
                    self.provider_id().as_str(),
                    "textDocument/references",
                    "rust-analyzer-lib returned a reference set for an unknown target",
                ));
            };

            for location in reference_set.references {
                let occurrence_range = text_range_from_lsp(location.range);
                let Some(file_context) = symbol_index.file_context(&location.file_path) else {
                    skipped_external += 1;
                    continue;
                };

                let source_symbol = file_context.deepest_symbol_containing(occurrence_range);
                let (source_symbol_key, source_resolution, confidence, confidence_score) =
                    match source_symbol {
                        Some(symbol) => (
                            symbol.symbol_key.clone(),
                            "symbol".to_string(),
                            "EXTRACTED".to_string(),
                            1.0,
                        ),
                        None => {
                            file_fallbacks += 1;
                            (
                                file_context.source_file.file_symbol_key.clone(),
                                "file_fallback".to_string(),
                                "AMBIGUOUS".to_string(),
                                0.6,
                            )
                        }
                    };
                let enclosing_symbol_key = source_symbol.map(|symbol| symbol.symbol_key.clone());
                let occurrence = ReferenceOccurrence {
                    file_uri: file_context.source_file.uri.clone(),
                    file_relative_path: file_context.source_file.relative_path.clone(),
                    file_symbol_key: file_context.source_file.file_symbol_key.clone(),
                    range: occurrence_range,
                    enclosing_symbol_key,
                    raw_json: json!({
                        "lsp_method": "textDocument/references",
                        "provider_range": location.range,
                        "route": RouteName::RUST_REFERENCES.as_str(),
                        "source_resolution": source_resolution,
                        "source_symbol_key": source_symbol_key,
                        "target_name": target_context.symbol.name,
                        "target_symbol_key": target_context.symbol.symbol_key,
                    }),
                };
                let group_key = (
                    source_symbol_key.clone(),
                    target_context.symbol.symbol_key.clone(),
                    source_resolution.clone(),
                );
                grouped_references
                    .entry(group_key)
                    .or_insert_with(|| ReferenceGroup {
                        source_symbol_key,
                        target_symbol_key: target_context.symbol.symbol_key.clone(),
                        source_resolution,
                        confidence,
                        confidence_score,
                        occurrences: Vec::new(),
                    })
                    .occurrences
                    .push(occurrence);
            }
        }

        let references = grouped_references
            .into_values()
            .map(|group| ExtractedReference {
                provider: self.provider_id(),
                source_symbol_key: group.source_symbol_key,
                target_symbol_key: group.target_symbol_key,
                source_resolution: group.source_resolution,
                confidence: group.confidence,
                confidence_score: group.confidence_score,
                occurrences: group.occurrences,
                raw_json: json!({
                    "lsp_method": "textDocument/references",
                    "route": RouteName::RUST_REFERENCES.as_str(),
                }),
            })
            .collect::<Vec<_>>();
        let reference_occurrences = references
            .iter()
            .map(|reference| reference.occurrences.len())
            .sum();
        let summary = ReferenceRouteSummary {
            targets_queried: reference_targets.len(),
            reference_edges: references.len(),
            reference_occurrences,
            file_fallbacks,
            skipped_external,
        };

        Ok(ReferenceBatchExtraction {
            provider: self.provider_id(),
            provider_version: document_symbols.provider_version.clone(),
            workspace_fingerprint,
            document_symbols,
            references,
            summary,
            raw_metadata: json!({
                "facade": "rust-analyzer-lib",
                "lsp_method": "textDocument/references",
            }),
        })
    }

    async fn run_calls(&self, request: CallBatchRequest) -> ExtractResult<CallBatchExtraction> {
        let document_request =
            validate_document_symbol_batch_request(DocumentSymbolBatchRequest {
                workspace_root: request.workspace_root,
                package_path: request.package_path,
                file_paths: request.file_paths,
            })?;
        let document_symbols = self.run_batch(document_request.clone()).await?;
        let callable_contexts = callable_target_contexts(&document_request, &document_symbols)?;
        let call_targets = callable_contexts
            .iter()
            .map(|context| context.target.clone())
            .collect::<Vec<_>>();
        let call_sets = rust_analyzer_lib::outgoing_calls_for_symbols(
            &document_request.workspace_root,
            &call_targets,
        )
        .map_err(|source| facade_error("rust-analyzer-lib outgoing_calls_for_symbols", source))?;
        let workspace_fingerprint = workspace_fingerprint(&document_symbols);
        let symbol_index = SymbolIndex::new(&document_request.workspace_root, &document_symbols)?;
        let caller_by_range = callable_contexts
            .into_iter()
            .map(|context| {
                (
                    target_key(&context.target.file_path, context.target.selection_range),
                    context,
                )
            })
            .collect::<HashMap<_, _>>();
        let mut grouped_calls = BTreeMap::new();
        let mut skipped_external_targets = 0;
        let mut skipped_unresolved_targets = 0;
        let mut skipped_non_callable_prepare_items = 0;

        for call_set in call_sets {
            skipped_non_callable_prepare_items += call_set.skipped_non_callable_prepare_items;
            let Some(caller_context) = caller_by_range.get(&target_key(
                &call_set.caller_file_path,
                call_set.caller_selection_range,
            )) else {
                return Err(ExtractError::response_shape(
                    self.provider_id().as_str(),
                    "callHierarchy/outgoingCalls",
                    "rust-analyzer-lib returned a call set for an unknown caller",
                ));
            };
            let Some(caller_file_context) = symbol_index.file_context(&call_set.caller_file_path)
            else {
                return Err(ExtractError::response_shape(
                    self.provider_id().as_str(),
                    "callHierarchy/outgoingCalls",
                    "call caller file was not in the current document-symbol batch",
                ));
            };

            for outgoing_call in call_set.outgoing_calls {
                if !outgoing_call
                    .target_file_path
                    .starts_with(&document_request.workspace_root)
                {
                    skipped_external_targets += 1;
                    continue;
                }

                let target_range = text_range_from_lsp(outgoing_call.target_range);
                let target_selection_range =
                    text_range_from_lsp(outgoing_call.target_selection_range);
                let Some(mapped_target) = symbol_index.call_target_mapping(
                    &outgoing_call.target_file_path,
                    &outgoing_call.target_name,
                    &outgoing_call.target_kind,
                    target_range,
                    target_selection_range,
                ) else {
                    skipped_unresolved_targets += 1;
                    continue;
                };

                let group_key = (
                    caller_context.symbol.symbol_key.clone(),
                    mapped_target.symbol.symbol_key.clone(),
                    "direct".to_string(),
                );
                let group = grouped_calls.entry(group_key).or_insert_with(|| CallGroup {
                    caller_symbol_key: caller_context.symbol.symbol_key.clone(),
                    callee_symbol_key: mapped_target.symbol.symbol_key.clone(),
                    context: "direct".to_string(),
                    occurrences: Vec::new(),
                });

                for callsite_range in outgoing_call.callsite_ranges {
                    let occurrence_range = text_range_from_lsp(callsite_range);
                    group.occurrences.push(CallOccurrence {
                        file_uri: caller_file_context.source_file.uri.clone(),
                        file_relative_path: caller_file_context.source_file.relative_path.clone(),
                        file_symbol_key: caller_file_context.source_file.file_symbol_key.clone(),
                        range: occurrence_range,
                        enclosing_symbol_key: caller_context.symbol.symbol_key.clone(),
                        raw_json: json!({
                            "lsp_method": "callHierarchy/outgoingCalls",
                            "route": RouteName::RUST_CALLS.as_str(),
                            "caller_name": caller_context.symbol.name,
                            "caller_symbol_key": caller_context.symbol.symbol_key,
                            "callee_name": mapped_target.symbol.name,
                            "callee_symbol_key": mapped_target.symbol.symbol_key,
                            "provider_target": {
                                "file_path": outgoing_call.target_file_path,
                                "name": outgoing_call.target_name,
                                "kind": outgoing_call.target_kind,
                                "range": outgoing_call.target_range,
                                "selection_range": outgoing_call.target_selection_range,
                            },
                            "from_range": callsite_range,
                            "target_resolution_mode": mapped_target.resolution_mode,
                        }),
                    });
                }
            }
        }

        let calls = grouped_calls
            .into_values()
            .map(|group| ExtractedCall {
                provider: self.provider_id(),
                caller_symbol_key: group.caller_symbol_key,
                callee_symbol_key: group.callee_symbol_key,
                context: group.context,
                confidence: "EXTRACTED".to_string(),
                confidence_score: 1.0,
                occurrences: group.occurrences,
                raw_json: json!({
                    "lsp_method": "callHierarchy/outgoingCalls",
                    "route": RouteName::RUST_CALLS.as_str(),
                }),
            })
            .collect::<Vec<_>>();
        let call_occurrences = calls.iter().map(|call| call.occurrences.len()).sum();
        let summary = CallRouteSummary {
            callable_nodes: call_targets.len(),
            call_edges: calls.len(),
            call_occurrences,
            skipped_external_targets,
            skipped_unresolved_targets,
            skipped_non_callable_prepare_items,
        };

        Ok(CallBatchExtraction {
            provider: self.provider_id(),
            provider_version: document_symbols.provider_version.clone(),
            workspace_fingerprint,
            document_symbols,
            calls,
            summary,
            raw_metadata: json!({
                "facade": "rust-analyzer-lib",
                "lsp_method": "callHierarchy/outgoingCalls",
            }),
        })
    }
}

impl Default for RustAnalyzerProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentSymbolProvider for RustAnalyzerProvider {
    fn provider_id(&self) -> ProviderId {
        ProviderId::rust_analyzer()
    }

    fn language(&self) -> GraphLanguage {
        GraphLanguage::Rust
    }

    async fn extract_document_symbols(
        &self,
        request: DocumentSymbolRequest,
    ) -> ExtractResult<DocumentSymbolExtraction> {
        self.run(request).await
    }
}

fn facade_error(
    context: &'static str,
    source: rust_analyzer_lib::RustAnalyzerLibError,
) -> ExtractError {
    ExtractError::rust_analyzer_lib(context, source)
}

#[derive(Debug, Clone)]
struct ReferenceTargetContext {
    target: rust_analyzer_lib::ResolvedReferenceTarget,
    symbol: ExtractedSymbol,
}

#[derive(Debug)]
struct ReferenceGroup {
    source_symbol_key: String,
    target_symbol_key: String,
    source_resolution: String,
    confidence: String,
    confidence_score: f64,
    occurrences: Vec<ReferenceOccurrence>,
}

#[derive(Debug, Clone)]
struct CallableTargetContext {
    target: rust_analyzer_lib::ResolvedCallTarget,
    symbol: ExtractedSymbol,
}

#[derive(Debug)]
struct CallGroup {
    caller_symbol_key: String,
    callee_symbol_key: String,
    context: String,
    occurrences: Vec<CallOccurrence>,
}

#[derive(Debug, Clone)]
struct MappedCallTarget {
    symbol: ExtractedSymbol,
    resolution_mode: &'static str,
}

#[derive(Debug)]
struct FileSymbolContext {
    source_file: SourceFile,
    symbols: Vec<ExtractedSymbol>,
}

impl FileSymbolContext {
    fn deepest_symbol_containing(
        &self,
        range: semantic_graph_store::TextRange,
    ) -> Option<&ExtractedSymbol> {
        self.symbols
            .iter()
            .filter(|symbol| contains_range(symbol.range, range))
            .min_by_key(|symbol| range_size(symbol.range))
    }
}

#[derive(Debug)]
struct SymbolIndex {
    files_by_path: HashMap<PathBuf, FileSymbolContext>,
}

impl SymbolIndex {
    fn new(
        workspace_root: &Path,
        extraction: &DocumentSymbolBatchExtraction,
    ) -> ExtractResult<Self> {
        let mut files_by_path = HashMap::new();
        for file_extraction in &extraction.extractions {
            let file_path = workspace_root.join(&file_extraction.source_file.relative_path);
            let file_path = file_path.canonicalize().map_err(|source| {
                ExtractError::io(
                    "canonicalize source file for reference mapping",
                    Some(file_path.clone()),
                    source,
                )
            })?;
            files_by_path.insert(
                file_path,
                FileSymbolContext {
                    source_file: file_extraction.source_file.clone(),
                    symbols: file_extraction.symbols.clone(),
                },
            );
        }

        Ok(Self { files_by_path })
    }

    fn file_context(&self, file_path: &Path) -> Option<&FileSymbolContext> {
        self.files_by_path.get(file_path)
    }

    fn call_target_mapping(
        &self,
        file_path: &Path,
        target_name: &str,
        target_kind: &str,
        target_range: semantic_graph_store::TextRange,
        target_selection_range: semantic_graph_store::TextRange,
    ) -> Option<MappedCallTarget> {
        let file_context = self.file_context(file_path)?;

        if let Some(symbol) = file_context
            .symbols
            .iter()
            .find(|symbol| symbol.selection_range == target_selection_range)
        {
            return Some(MappedCallTarget {
                symbol: symbol.clone(),
                resolution_mode: "exact_selection_range",
            });
        }

        if let Some(symbol) = file_context
            .symbols
            .iter()
            .filter(|symbol| symbol.name == target_name)
            .filter(|symbol| contains_range(symbol.range, target_range))
            .min_by_key(|symbol| range_size(symbol.range))
        {
            return Some(MappedCallTarget {
                symbol: symbol.clone(),
                resolution_mode: "containing_name_range",
            });
        }

        file_context
            .symbols
            .iter()
            .filter(|symbol| symbol.name == target_name)
            .filter(|symbol| call_kind_matches(&symbol.kind, target_kind))
            .min_by_key(|symbol| range_distance(symbol.selection_range, target_selection_range))
            .map(|symbol| MappedCallTarget {
                symbol: symbol.clone(),
                resolution_mode: "nearest_name_kind_range",
            })
    }
}

fn reference_target_contexts(
    request: &DocumentSymbolBatchRequest,
    extraction: &DocumentSymbolBatchExtraction,
) -> ExtractResult<Vec<ReferenceTargetContext>> {
    let mut targets = Vec::new();
    for file_extraction in &extraction.extractions {
        let file_path = request
            .workspace_root
            .join(&file_extraction.source_file.relative_path)
            .canonicalize()
            .map_err(|source| {
                ExtractError::io(
                    "canonicalize reference target source file",
                    Some(
                        request
                            .workspace_root
                            .join(&file_extraction.source_file.relative_path),
                    ),
                    source,
                )
            })?;

        for symbol in &file_extraction.symbols {
            if !is_reference_target_kind(&symbol.kind) {
                continue;
            }

            targets.push(ReferenceTargetContext {
                target: rust_analyzer_lib::ResolvedReferenceTarget {
                    file_path: file_path.clone(),
                    selection_range: lsp_range_from_text_range(symbol.selection_range)?,
                    name: symbol.name.clone(),
                },
                symbol: symbol.clone(),
            });
        }
    }

    Ok(targets)
}

fn callable_target_contexts(
    request: &DocumentSymbolBatchRequest,
    extraction: &DocumentSymbolBatchExtraction,
) -> ExtractResult<Vec<CallableTargetContext>> {
    let mut targets = Vec::new();
    for file_extraction in &extraction.extractions {
        let file_path = request
            .workspace_root
            .join(&file_extraction.source_file.relative_path)
            .canonicalize()
            .map_err(|source| {
                ExtractError::io(
                    "canonicalize call target source file",
                    Some(
                        request
                            .workspace_root
                            .join(&file_extraction.source_file.relative_path),
                    ),
                    source,
                )
            })?;

        for symbol in &file_extraction.symbols {
            if !is_callable_symbol_kind(&symbol.kind) {
                continue;
            }

            targets.push(CallableTargetContext {
                target: rust_analyzer_lib::ResolvedCallTarget {
                    file_path: file_path.clone(),
                    selection_range: lsp_range_from_text_range(symbol.selection_range)?,
                    name: symbol.name.clone(),
                },
                symbol: symbol.clone(),
            });
        }
    }

    Ok(targets)
}

fn lsp_range_from_text_range(range: semantic_graph_store::TextRange) -> ExtractResult<Range> {
    Ok(Range {
        start: lsp_position(range.start_line, range.start_col)?,
        end: lsp_position(range.end_line, range.end_col)?,
    })
}

fn lsp_position(line: i64, character: i64) -> ExtractResult<Position> {
    let line = u32::try_from(line).map_err(|_| {
        ExtractError::response_shape(
            ProviderId::rust_analyzer().as_str(),
            "textDocument/references",
            "document-symbol range line could not be converted to LSP position",
        )
    })?;
    let character = u32::try_from(character).map_err(|_| {
        ExtractError::response_shape(
            ProviderId::rust_analyzer().as_str(),
            "textDocument/references",
            "document-symbol range column could not be converted to LSP position",
        )
    })?;

    Ok(Position { line, character })
}

fn is_reference_target_kind(kind: &str) -> bool {
    matches!(
        kind,
        "function"
            | "method"
            | "struct"
            | "enum"
            | "enum_member"
            | "interface"
            | "field"
            | "constant"
            | "type_alias"
            | "type_parameter"
            | "module"
    )
}

fn is_callable_symbol_kind(kind: &str) -> bool {
    matches!(kind, "function" | "method" | "constructor")
}

fn call_kind_matches(symbol_kind: &str, target_kind: &str) -> bool {
    symbol_kind == target_kind
        || matches!(
            (symbol_kind, target_kind),
            ("method", "function") | ("function", "method")
        )
}

fn target_key(file_path: &Path, range: Range) -> String {
    format!(
        "{}:{}:{}-{}:{}",
        file_path.display(),
        range.start.line,
        range.start.character,
        range.end.line,
        range.end.character
    )
}

fn range_distance(
    left: semantic_graph_store::TextRange,
    right: semantic_graph_store::TextRange,
) -> i64 {
    (left.start_line - right.start_line).abs() * 1_000_000
        + (left.start_col - right.start_col).abs()
        + (left.end_line - right.end_line).abs() * 1_000_000
        + (left.end_col - right.end_col).abs()
}

fn contains_range(
    container: semantic_graph_store::TextRange,
    inner: semantic_graph_store::TextRange,
) -> bool {
    range_start_le(container, inner) && range_end_ge(container, inner)
}

fn range_start_le(
    left: semantic_graph_store::TextRange,
    right: semantic_graph_store::TextRange,
) -> bool {
    (left.start_line, left.start_col) <= (right.start_line, right.start_col)
}

fn range_end_ge(
    left: semantic_graph_store::TextRange,
    right: semantic_graph_store::TextRange,
) -> bool {
    (left.end_line, left.end_col) >= (right.end_line, right.end_col)
}

fn range_size(range: semantic_graph_store::TextRange) -> i64 {
    (range.end_line - range.start_line) * 1_000_000 + (range.end_col - range.start_col)
}

fn workspace_fingerprint(extraction: &DocumentSymbolBatchExtraction) -> String {
    let mut entries = extraction
        .extractions
        .iter()
        .map(|file_extraction| {
            format!(
                "{}:{}",
                file_extraction.source_file.relative_path,
                file_extraction
                    .source_file
                    .content_hash
                    .as_deref()
                    .unwrap_or_default()
            )
        })
        .collect::<Vec<_>>();
    entries.sort();

    let mut hasher = Sha256::new();
    for entry in entries {
        hasher.update(entry.as_bytes());
        hasher.update(b"\n");
    }

    hex::encode(hasher.finalize())
}
