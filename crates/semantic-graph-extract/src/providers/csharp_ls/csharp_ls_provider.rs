use crate::{
    ExtractError, ExtractResult,
    document_symbols::mapper::text_range_from_lsp,
    document_symbols::paths::validate_document_symbol_batch_request,
    model::{
        CallBatchExtraction, CallOccurrence, CallRouteSummary, DocumentSymbolBatchExtraction,
        DocumentSymbolBatchRequest, DocumentSymbolRequest, ExtractedCall, ExtractedReference,
        ExtractedSymbol, GraphLanguage, ProviderId, ReferenceBatchExtraction, ReferenceOccurrence,
        ReferenceRouteSummary, RouteName, SourceFile,
    },
    providers::csharp_ls::CSharpDocumentSymbolMapper,
};

use lsp_types::{DocumentSymbolResponse, Position, Range};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CSharpLsProvider;

impl CSharpLsProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn provider_id(&self) -> ProviderId {
        ProviderId::csharp_language_server()
    }

    pub fn language(&self) -> GraphLanguage {
        GraphLanguage::CSharp
    }

    pub fn discover_csharp_solution_source_files(
        &self,
        solution_path: &Path,
    ) -> ExtractResult<Vec<PathBuf>> {
        let model = csharp_ls_lib::load_solution(solution_path)
            .map_err(|source| ExtractError::csharp_ls_lib("load C# solution", source))?;
        Ok(csharp_ls_lib::solution_source_files(&model))
    }

    pub fn discover_csharp_project_source_files(
        &self,
        solution_path: &Path,
        project_or_root: &Path,
    ) -> ExtractResult<Vec<PathBuf>> {
        let model = csharp_ls_lib::load_solution(solution_path)
            .map_err(|source| ExtractError::csharp_ls_lib("load C# solution", source))?;
        csharp_ls_lib::project_source_files(&model, project_or_root)
            .map_err(|source| ExtractError::csharp_ls_lib("discover C# project files", source))
    }

    pub fn map_document_symbol_items(
        &self,
        request: DocumentSymbolBatchRequest,
        document_symbols: Vec<(PathBuf, Vec<lsp_types::DocumentSymbol>)>,
    ) -> ExtractResult<DocumentSymbolBatchExtraction> {
        let request = validate_document_symbol_batch_request(request)?;
        self.map_document_symbol_items_with_version(
            request,
            csharp_ls_lib::provider_version(),
            document_symbols,
        )
    }

    pub fn reference_targets_for_document_symbols(
        &self,
        request: &DocumentSymbolBatchRequest,
        document_symbols: &DocumentSymbolBatchExtraction,
    ) -> ExtractResult<Vec<csharp_ls_lib::ResolvedReferenceTarget>> {
        reference_target_contexts(request, document_symbols).map(|contexts| {
            contexts
                .into_iter()
                .map(|context| context.target)
                .collect::<Vec<_>>()
        })
    }

    pub fn call_targets_for_document_symbols(
        &self,
        request: &DocumentSymbolBatchRequest,
        document_symbols: &DocumentSymbolBatchExtraction,
    ) -> ExtractResult<Vec<csharp_ls_lib::ResolvedCallTarget>> {
        callable_target_contexts(request, document_symbols).map(|contexts| {
            contexts
                .into_iter()
                .map(|context| context.target)
                .collect::<Vec<_>>()
        })
    }

    pub fn map_reference_sets(
        &self,
        document_request: &DocumentSymbolBatchRequest,
        document_symbols: DocumentSymbolBatchExtraction,
        reference_sets: Vec<csharp_ls_lib::ResolvedReferenceSet>,
        targets_queried: usize,
    ) -> ExtractResult<ReferenceBatchExtraction> {
        let target_contexts = reference_target_contexts(document_request, &document_symbols)?;
        let workspace_fingerprint = workspace_fingerprint(&document_symbols);
        let symbol_index = SymbolIndex::new(&document_request.workspace_root, &document_symbols)?;
        let target_by_range = target_contexts
            .into_iter()
            .map(|context| {
                (
                    target_key(&context.target.file_path, context.target.selection_range),
                    context,
                )
            })
            .collect::<HashMap<_, _>>();
        let mut grouped_references = BTreeMap::new();
        let mut skipped_external = 0;
        let mut file_fallbacks = 0;

        for reference_set in reference_sets {
            let Some(target_context) = target_by_range.get(&target_key(
                &reference_set.target_file_path,
                reference_set.target_selection_range,
            )) else {
                return Err(ExtractError::response_shape(
                    self.provider_id().as_str(),
                    "textDocument/references",
                    "csharp-ls-lib returned a reference set for an unknown target",
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
                        "route": RouteName::CSHARP_REFERENCES.as_str(),
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
                    "route": RouteName::CSHARP_REFERENCES.as_str(),
                }),
            })
            .collect::<Vec<_>>();
        let reference_occurrences = references
            .iter()
            .map(|reference| reference.occurrences.len())
            .sum();
        let summary = ReferenceRouteSummary {
            targets_queried,
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
                "facade": "csharp-ls-lib",
                "lsp_method": "textDocument/references",
            }),
        })
    }

    pub fn map_incoming_call_sets(
        &self,
        document_request: &DocumentSymbolBatchRequest,
        document_symbols: DocumentSymbolBatchExtraction,
        incoming_call_sets: Vec<csharp_ls_lib::ResolvedIncomingCallSet>,
        callable_nodes: usize,
    ) -> ExtractResult<CallBatchExtraction> {
        let callable_contexts = callable_target_contexts(document_request, &document_symbols)?;
        let workspace_fingerprint = workspace_fingerprint(&document_symbols);
        let symbol_index = SymbolIndex::new(&document_request.workspace_root, &document_symbols)?;
        let target_by_range = callable_contexts
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

        for call_set in incoming_call_sets {
            skipped_non_callable_prepare_items += call_set.skipped_non_callable_prepare_items;
            let Some(callee_context) = target_by_range.get(&target_key(
                &call_set.target_file_path,
                call_set.target_selection_range,
            )) else {
                return Err(ExtractError::response_shape(
                    self.provider_id().as_str(),
                    "callHierarchy/incomingCalls",
                    "csharp-ls-lib returned an incoming call set for an unknown target",
                ));
            };

            for incoming_call in call_set.incoming_calls {
                if !incoming_call
                    .caller_file_path
                    .starts_with(&document_request.workspace_root)
                {
                    skipped_external_targets += 1;
                    continue;
                }
                let Some(caller_file_context) =
                    symbol_index.file_context(&incoming_call.caller_file_path)
                else {
                    skipped_external_targets += 1;
                    continue;
                };
                let caller_range = text_range_from_lsp(incoming_call.caller_range);
                let caller_selection_range =
                    text_range_from_lsp(incoming_call.caller_selection_range);
                let Some(mapped_caller) = symbol_index.call_target_mapping(
                    &incoming_call.caller_file_path,
                    &incoming_call.caller_name,
                    &incoming_call.caller_kind,
                    caller_range,
                    caller_selection_range,
                ) else {
                    skipped_unresolved_targets += 1;
                    continue;
                };

                let group_key = (
                    mapped_caller.symbol.symbol_key.clone(),
                    callee_context.symbol.symbol_key.clone(),
                    "direct".to_string(),
                );
                let group = grouped_calls.entry(group_key).or_insert_with(|| CallGroup {
                    caller_symbol_key: mapped_caller.symbol.symbol_key.clone(),
                    callee_symbol_key: callee_context.symbol.symbol_key.clone(),
                    context: "direct".to_string(),
                    occurrences: Vec::new(),
                });

                for from_range in incoming_call.from_ranges {
                    let occurrence_range = text_range_from_lsp(from_range);
                    group.occurrences.push(CallOccurrence {
                        file_uri: caller_file_context.source_file.uri.clone(),
                        file_relative_path: caller_file_context.source_file.relative_path.clone(),
                        file_symbol_key: caller_file_context.source_file.file_symbol_key.clone(),
                        range: occurrence_range,
                        enclosing_symbol_key: mapped_caller.symbol.symbol_key.clone(),
                        raw_json: json!({
                            "lsp_method": "callHierarchy/incomingCalls",
                            "route": RouteName::CSHARP_CALLS.as_str(),
                            "caller_name": mapped_caller.symbol.name,
                            "caller_symbol_key": mapped_caller.symbol.symbol_key,
                            "callee_name": callee_context.symbol.name,
                            "callee_symbol_key": callee_context.symbol.symbol_key,
                            "from_range": from_range,
                            "caller_resolution_mode": mapped_caller.resolution_mode,
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
                    "lsp_method": "callHierarchy/incomingCalls",
                    "route": RouteName::CSHARP_CALLS.as_str(),
                }),
            })
            .collect::<Vec<_>>();
        let call_occurrences = calls.iter().map(|call| call.occurrences.len()).sum();
        let summary = CallRouteSummary {
            callable_nodes,
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
                "facade": "csharp-ls-lib",
                "lsp_method": "callHierarchy/incomingCalls",
            }),
        })
    }

    fn map_document_symbol_items_with_version(
        &self,
        request: DocumentSymbolBatchRequest,
        provider_version: Option<String>,
        document_symbols: Vec<(PathBuf, Vec<lsp_types::DocumentSymbol>)>,
    ) -> ExtractResult<DocumentSymbolBatchExtraction> {
        let mut extractions = Vec::with_capacity(document_symbols.len());

        for (file_path, symbols) in document_symbols {
            let response = DocumentSymbolResponse::Nested(symbols.clone());
            let extraction = CSharpDocumentSymbolMapper::map_response(
                DocumentSymbolRequest {
                    workspace_root: request.workspace_root.clone(),
                    package_path: request.package_path.clone(),
                    file_path: file_path.clone(),
                },
                response,
                provider_version.clone(),
                json!({
                    "facade": "csharp-ls-lib",
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
                "facade": "csharp-ls-lib",
                "file_count": request.file_paths.len(),
            }),
        })
    }
}

impl Default for CSharpLsProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
struct ReferenceTargetContext {
    target: csharp_ls_lib::ResolvedReferenceTarget,
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
    target: csharp_ls_lib::ResolvedCallTarget,
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
                    "canonicalize C# source file for relation mapping",
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
        let file_path = canonical_file_path(request, file_extraction)?;

        for symbol in &file_extraction.symbols {
            if !is_reference_target_kind(&symbol.kind) {
                continue;
            }

            targets.push(ReferenceTargetContext {
                target: csharp_ls_lib::ResolvedReferenceTarget {
                    file_path: file_path.clone(),
                    selection_range: lsp_range_from_text_range(symbol.selection_range)?,
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
        let file_path = canonical_file_path(request, file_extraction)?;

        for symbol in &file_extraction.symbols {
            if !is_callable_symbol_kind(&symbol.kind) {
                continue;
            }

            targets.push(CallableTargetContext {
                target: csharp_ls_lib::ResolvedCallTarget {
                    file_path: file_path.clone(),
                    selection_range: lsp_range_from_text_range(symbol.selection_range)?,
                },
                symbol: symbol.clone(),
            });
        }
    }

    Ok(targets)
}

fn canonical_file_path(
    request: &DocumentSymbolBatchRequest,
    file_extraction: &crate::model::DocumentSymbolExtraction,
) -> ExtractResult<PathBuf> {
    let file_path = request
        .workspace_root
        .join(&file_extraction.source_file.relative_path);
    file_path.canonicalize().map_err(|source| {
        ExtractError::io(
            "canonicalize C# relation target source file",
            Some(file_path),
            source,
        )
    })
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
            ProviderId::csharp_language_server().as_str(),
            "textDocument/references",
            "document-symbol range line could not be converted to LSP position",
        )
    })?;
    let character = u32::try_from(character).map_err(|_| {
        ExtractError::response_shape(
            ProviderId::csharp_language_server().as_str(),
            "textDocument/references",
            "document-symbol range column could not be converted to LSP position",
        )
    })?;

    Ok(Position { line, character })
}

fn is_reference_target_kind(kind: &str) -> bool {
    matches!(
        kind,
        "method"
            | "constructor"
            | "class"
            | "struct"
            | "enum"
            | "enum_member"
            | "interface"
            | "field"
            | "property"
            | "event"
            | "module"
            | "variable"
            | "constant"
    )
}

fn is_callable_symbol_kind(kind: &str) -> bool {
    matches!(
        kind,
        "method" | "constructor" | "property" | "field" | "event"
    )
}

fn call_kind_matches(symbol_kind: &str, target_kind: &str) -> bool {
    symbol_kind == target_kind
        || matches!(
            (symbol_kind, target_kind),
            ("method", "Method")
                | ("method", "METHOD")
                | ("constructor", "Constructor")
                | ("constructor", "CONSTRUCTOR")
                | ("property", "Property")
                | ("property", "PROPERTY")
                | ("field", "Field")
                | ("field", "FIELD")
                | ("event", "Event")
                | ("event", "EVENT")
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
