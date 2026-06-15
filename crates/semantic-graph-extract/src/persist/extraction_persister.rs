use crate::{
    ExtractError, ExtractResult,
    document_symbols::paths::basename_from_relative_path,
    model::{
        CallBatchExtraction, DocumentSymbolBatchExtraction, DocumentSymbolExtraction,
        ExtractedCall, ExtractedReference, ReferenceBatchExtraction, RouteName, RouteScope,
    },
    persist::{PersistenceSummary, ScopedRoute},
};

use semantic_graph_db_manager::{
    CloseStaleRouteInput, EdgeEvidenceInput, EdgeInput, FileInput, NodeInput, OccurrenceInput,
    RouteObservationInput, RouteStatusCompleteInput, RouteStatusFailInput, RouteStatusStartInput,
    WriteHandle, node_id,
};

use serde_json::{Value, json};
use std::collections::HashMap;

pub struct ExtractionPersister;

impl ExtractionPersister {
    pub async fn persist_document_symbols(
        &self,
        store: &WriteHandle,
        workspace_root_uri: &str,
        extraction: &DocumentSymbolExtraction,
    ) -> ExtractResult<PersistenceSummary> {
        let workspace_id = store
            .create_workspace(
                workspace_root_uri,
                extraction.source_file.language.workspace_kind(),
            )
            .await
            .map_err(ExtractError::storage)?;
        let run_id = store
            .start_run(
                workspace_id,
                extraction.provider.as_str(),
                extraction.provider_version.as_deref(),
                None,
            )
            .await
            .map_err(ExtractError::storage)?;

        let result = self
            .persist_after_run_started(store, workspace_id, run_id, extraction)
            .await;

        match result {
            Ok(summary) => {
                store
                    .finish_run(run_id, "complete")
                    .await
                    .map_err(ExtractError::storage)?;
                Ok(summary)
            }
            Err(error) => {
                let finish_result = store.finish_run(run_id, "failed").await;
                if let Err(finish_error) = finish_result {
                    return Err(ExtractError::storage(finish_error));
                }
                Err(error)
            }
        }
    }

    pub async fn persist_document_symbol_batch(
        &self,
        store: &WriteHandle,
        workspace_root_uri: &str,
        extraction: &DocumentSymbolBatchExtraction,
    ) -> ExtractResult<PersistenceSummary> {
        let first_extraction = extraction.extractions.first().ok_or_else(|| {
            ExtractError::response_shape(
                extraction.provider.as_str(),
                "textDocument/documentSymbol",
                "document symbol batch contained no files",
            )
        })?;
        let workspace_id = store
            .create_workspace(
                workspace_root_uri,
                first_extraction.source_file.language.workspace_kind(),
            )
            .await
            .map_err(ExtractError::storage)?;
        let run_id = store
            .start_run(
                workspace_id,
                extraction.provider.as_str(),
                extraction.provider_version.as_deref(),
                None,
            )
            .await
            .map_err(ExtractError::storage)?;

        let result = self
            .persist_batch_after_run_started(store, workspace_id, run_id, extraction)
            .await;

        match result {
            Ok(summary) => {
                store
                    .finish_run(run_id, "complete")
                    .await
                    .map_err(ExtractError::storage)?;
                Ok(summary)
            }
            Err(error) => {
                let finish_result = store.finish_run(run_id, "failed").await;
                if let Err(finish_error) = finish_result {
                    return Err(ExtractError::storage(finish_error));
                }
                Err(error)
            }
        }
    }

    pub async fn persist_reference_batch_with_document_symbols(
        &self,
        store: &WriteHandle,
        workspace_root_uri: &str,
        extraction: &ReferenceBatchExtraction,
    ) -> ExtractResult<PersistenceSummary> {
        let first_extraction =
            extraction
                .document_symbols
                .extractions
                .first()
                .ok_or_else(|| {
                    ExtractError::response_shape(
                        extraction.provider.as_str(),
                        "textDocument/references",
                        "reference batch contained no document-symbol files",
                    )
                })?;
        let workspace_id = store
            .create_workspace(
                workspace_root_uri,
                first_extraction.source_file.language.workspace_kind(),
            )
            .await
            .map_err(ExtractError::storage)?;
        let run_id = store
            .start_run(
                workspace_id,
                extraction.provider.as_str(),
                extraction.provider_version.as_deref(),
                None,
            )
            .await
            .map_err(ExtractError::storage)?;

        let result = self
            .persist_reference_batch_with_document_symbols_after_run_started(
                store,
                workspace_id,
                run_id,
                workspace_root_uri,
                extraction,
            )
            .await;

        match result {
            Ok(summary) => {
                store
                    .finish_run(run_id, "complete")
                    .await
                    .map_err(ExtractError::storage)?;
                Ok(summary)
            }
            Err(error) => {
                let finish_result = store.finish_run(run_id, "failed").await;
                if let Err(finish_error) = finish_result {
                    return Err(ExtractError::storage(finish_error));
                }
                Err(error)
            }
        }
    }

    pub async fn persist_reference_batch(
        &self,
        store: &WriteHandle,
        workspace_root_uri: &str,
        extraction: &ReferenceBatchExtraction,
    ) -> ExtractResult<PersistenceSummary> {
        let _first_extraction =
            extraction
                .document_symbols
                .extractions
                .first()
                .ok_or_else(|| {
                    ExtractError::response_shape(
                        extraction.provider.as_str(),
                        "textDocument/references",
                        "reference batch contained no document-symbol files",
                    )
                })?;
        let workspace_id = self
            .existing_workspace_id(
                store,
                workspace_root_uri,
                extraction.provider.as_str(),
                "textDocument/references",
            )
            .await?;
        let run_id = store
            .start_run(
                workspace_id,
                extraction.provider.as_str(),
                extraction.provider_version.as_deref(),
                None,
            )
            .await
            .map_err(ExtractError::storage)?;

        let result = self
            .persist_reference_batch_route_only_after_run_started(
                store,
                workspace_id,
                run_id,
                workspace_root_uri,
                extraction,
            )
            .await;

        match result {
            Ok(summary) => {
                store
                    .finish_run(run_id, "complete")
                    .await
                    .map_err(ExtractError::storage)?;
                Ok(summary)
            }
            Err(error) => {
                let finish_result = store.finish_run(run_id, "failed").await;
                if let Err(finish_error) = finish_result {
                    return Err(ExtractError::storage(finish_error));
                }
                Err(error)
            }
        }
    }

    pub async fn persist_reference_file_batch(
        &self,
        store: &WriteHandle,
        workspace_root_uri: &str,
        extraction: &ReferenceBatchExtraction,
    ) -> ExtractResult<PersistenceSummary> {
        let file_scope_key = single_file_scope_key(
            extraction.provider.as_str(),
            "textDocument/references",
            &extraction.document_symbols,
        )?;
        let workspace_id = self
            .existing_workspace_id(
                store,
                workspace_root_uri,
                extraction.provider.as_str(),
                "textDocument/references",
            )
            .await?;
        let run_id = store
            .start_run(
                workspace_id,
                extraction.provider.as_str(),
                extraction.provider_version.as_deref(),
                None,
            )
            .await
            .map_err(ExtractError::storage)?;

        let result = self
            .persist_reference_file_batch_after_run_started(
                store,
                workspace_id,
                run_id,
                &file_scope_key,
                extraction,
            )
            .await;

        match result {
            Ok(summary) => {
                store
                    .finish_run(run_id, "complete")
                    .await
                    .map_err(ExtractError::storage)?;
                Ok(summary)
            }
            Err(error) => {
                let finish_result = store.finish_run(run_id, "failed").await;
                if let Err(finish_error) = finish_result {
                    return Err(ExtractError::storage(finish_error));
                }
                Err(error)
            }
        }
    }

    async fn persist_reference_batch_route_only_after_run_started(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        run_id: i64,
        workspace_root_uri: &str,
        extraction: &ReferenceBatchExtraction,
    ) -> ExtractResult<PersistenceSummary> {
        let file_ids = self
            .existing_document_symbol_file_ids(
                store,
                workspace_id,
                extraction.provider.as_str(),
                "textDocument/references",
                &extraction.document_symbols,
            )
            .await?;
        self.validate_reference_nodes(store, workspace_id, extraction)
            .await?;

        store
            .start_route_status(RouteStatusStartInput {
                workspace_id,
                route: RouteName::RUST_REFERENCES.as_str(),
                scope: RouteScope::WORKSPACE.as_str(),
                scope_key: workspace_root_uri,
                file_id: None,
                provider: extraction.provider.as_str(),
                provider_version: extraction.provider_version.as_deref(),
                content_hash: Some(&extraction.workspace_fingerprint),
                run_id,
                diagnostics_json: json!({}),
            })
            .await
            .map_err(ExtractError::storage)?;

        let result = self
            .persist_references_after_scoped_route_started(
                store,
                workspace_id,
                run_id,
                ScopedRoute::workspace(workspace_root_uri),
                extraction,
                &file_ids,
            )
            .await;

        match result {
            Ok(mut summary) => {
                store
                    .complete_route_status(RouteStatusCompleteInput {
                        workspace_id,
                        route: RouteName::RUST_REFERENCES.as_str(),
                        scope: RouteScope::WORKSPACE.as_str(),
                        scope_key: workspace_root_uri,
                        provider: extraction.provider.as_str(),
                        provider_version: extraction.provider_version.as_deref(),
                        content_hash: Some(&extraction.workspace_fingerprint),
                        run_id,
                        diagnostics_json: json!({
                            "targets_queried": extraction.summary.targets_queried,
                            "reference_edges": summary.reference_edges,
                            "reference_occurrences": summary.reference_occurrences,
                            "file_fallbacks": extraction.summary.file_fallbacks,
                            "skipped_external": extraction.summary.skipped_external,
                        }),
                    })
                    .await
                    .map_err(ExtractError::storage)?;
                let stale_edges_closed = store
                    .close_stale_edges_for_route(CloseStaleRouteInput {
                        workspace_id,
                        run_id,
                        route: RouteName::RUST_REFERENCES.as_str(),
                        scope: RouteScope::WORKSPACE.as_str(),
                        scope_key: workspace_root_uri,
                        provider: extraction.provider.as_str(),
                    })
                    .await
                    .map_err(ExtractError::storage)?;

                summary.routes_complete = 1;
                summary.stale_edges_closed = stale_edges_closed as usize;

                Ok(summary)
            }
            Err(error) => {
                store
                    .fail_route_status(RouteStatusFailInput {
                        workspace_id,
                        route: RouteName::RUST_REFERENCES.as_str(),
                        scope: RouteScope::WORKSPACE.as_str(),
                        scope_key: workspace_root_uri,
                        provider: extraction.provider.as_str(),
                        run_id,
                        diagnostics_json: json!({
                            "kind": error.message(),
                            "error": error.to_string(),
                        }),
                    })
                    .await
                    .map_err(ExtractError::storage)?;
                Err(error)
            }
        }
    }

    async fn persist_reference_batch_with_document_symbols_after_run_started(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        run_id: i64,
        workspace_root_uri: &str,
        extraction: &ReferenceBatchExtraction,
    ) -> ExtractResult<PersistenceSummary> {
        let mut summary = self
            .persist_batch_after_run_started(
                store,
                workspace_id,
                run_id,
                &extraction.document_symbols,
            )
            .await?;
        let file_ids = self
            .document_symbol_file_ids(store, workspace_id, run_id, &extraction.document_symbols)
            .await?;

        store
            .start_route_status(RouteStatusStartInput {
                workspace_id,
                route: RouteName::RUST_REFERENCES.as_str(),
                scope: RouteScope::WORKSPACE.as_str(),
                scope_key: workspace_root_uri,
                file_id: None,
                provider: extraction.provider.as_str(),
                provider_version: extraction.provider_version.as_deref(),
                content_hash: Some(&extraction.workspace_fingerprint),
                run_id,
                diagnostics_json: json!({}),
            })
            .await
            .map_err(ExtractError::storage)?;

        let result = self
            .persist_references_after_scoped_route_started(
                store,
                workspace_id,
                run_id,
                ScopedRoute::workspace(workspace_root_uri),
                extraction,
                &file_ids,
            )
            .await;

        match result {
            Ok(reference_summary) => {
                store
                    .complete_route_status(RouteStatusCompleteInput {
                        workspace_id,
                        route: RouteName::RUST_REFERENCES.as_str(),
                        scope: RouteScope::WORKSPACE.as_str(),
                        scope_key: workspace_root_uri,
                        provider: extraction.provider.as_str(),
                        provider_version: extraction.provider_version.as_deref(),
                        content_hash: Some(&extraction.workspace_fingerprint),
                        run_id,
                        diagnostics_json: json!({
                            "targets_queried": extraction.summary.targets_queried,
                            "reference_edges": reference_summary.reference_edges,
                            "reference_occurrences": reference_summary.reference_occurrences,
                            "file_fallbacks": extraction.summary.file_fallbacks,
                            "skipped_external": extraction.summary.skipped_external,
                        }),
                    })
                    .await
                    .map_err(ExtractError::storage)?;
                let stale_edges_closed = store
                    .close_stale_edges_for_route(CloseStaleRouteInput {
                        workspace_id,
                        run_id,
                        route: RouteName::RUST_REFERENCES.as_str(),
                        scope: RouteScope::WORKSPACE.as_str(),
                        scope_key: workspace_root_uri,
                        provider: extraction.provider.as_str(),
                    })
                    .await
                    .map_err(ExtractError::storage)?;

                summary.edges += reference_summary.edges;
                summary.reference_edges += reference_summary.reference_edges;
                summary.occurrences += reference_summary.occurrences;
                summary.reference_occurrences += reference_summary.reference_occurrences;
                summary.evidence += reference_summary.evidence;
                summary.routes_complete += 1;
                summary.stale_edges_closed += stale_edges_closed as usize;

                Ok(summary)
            }
            Err(error) => {
                store
                    .fail_route_status(RouteStatusFailInput {
                        workspace_id,
                        route: RouteName::RUST_REFERENCES.as_str(),
                        scope: RouteScope::WORKSPACE.as_str(),
                        scope_key: workspace_root_uri,
                        provider: extraction.provider.as_str(),
                        run_id,
                        diagnostics_json: json!({
                            "kind": error.message(),
                            "error": error.to_string(),
                        }),
                    })
                    .await
                    .map_err(ExtractError::storage)?;
                Err(error)
            }
        }
    }

    async fn persist_reference_file_batch_after_run_started(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        run_id: i64,
        file_scope_key: &str,
        extraction: &ReferenceBatchExtraction,
    ) -> ExtractResult<PersistenceSummary> {
        let file_ids = self
            .existing_document_symbol_file_ids(
                store,
                workspace_id,
                extraction.provider.as_str(),
                "textDocument/references",
                &extraction.document_symbols,
            )
            .await?;
        self.validate_reference_nodes(store, workspace_id, extraction)
            .await?;
        let file_id = *file_ids.get(file_scope_key).ok_or_else(|| {
            ExtractError::response_shape(
                extraction.provider.as_str(),
                "textDocument/references",
                format!("source file {file_scope_key} is missing from the database"),
            )
        })?;
        let file_content_hash = single_file_content_hash(
            extraction.provider.as_str(),
            "textDocument/references",
            &extraction.document_symbols,
        )?;

        store
            .start_route_status(RouteStatusStartInput {
                workspace_id,
                route: RouteName::RUST_REFERENCES.as_str(),
                scope: RouteScope::FILE.as_str(),
                scope_key: file_scope_key,
                file_id: Some(file_id),
                provider: extraction.provider.as_str(),
                provider_version: extraction.provider_version.as_deref(),
                content_hash: file_content_hash.as_deref(),
                run_id,
                diagnostics_json: json!({}),
            })
            .await
            .map_err(ExtractError::storage)?;

        let result = self
            .persist_references_after_scoped_route_started(
                store,
                workspace_id,
                run_id,
                ScopedRoute::file(file_scope_key),
                extraction,
                &file_ids,
            )
            .await;

        match result {
            Ok(mut summary) => {
                store
                    .complete_route_status(RouteStatusCompleteInput {
                        workspace_id,
                        route: RouteName::RUST_REFERENCES.as_str(),
                        scope: RouteScope::FILE.as_str(),
                        scope_key: file_scope_key,
                        provider: extraction.provider.as_str(),
                        provider_version: extraction.provider_version.as_deref(),
                        content_hash: file_content_hash.as_deref(),
                        run_id,
                        diagnostics_json: json!({
                            "targets_queried": extraction.summary.targets_queried,
                            "reference_edges": summary.reference_edges,
                            "reference_occurrences": summary.reference_occurrences,
                            "file_fallbacks": extraction.summary.file_fallbacks,
                            "skipped_external": extraction.summary.skipped_external,
                        }),
                    })
                    .await
                    .map_err(ExtractError::storage)?;
                let stale_edges_closed = store
                    .close_stale_edges_for_route(CloseStaleRouteInput {
                        workspace_id,
                        run_id,
                        route: RouteName::RUST_REFERENCES.as_str(),
                        scope: RouteScope::FILE.as_str(),
                        scope_key: file_scope_key,
                        provider: extraction.provider.as_str(),
                    })
                    .await
                    .map_err(ExtractError::storage)?;

                summary.routes_complete = 1;
                summary.stale_edges_closed = stale_edges_closed as usize;

                Ok(summary)
            }
            Err(error) => {
                store
                    .fail_route_status(RouteStatusFailInput {
                        workspace_id,
                        route: RouteName::RUST_REFERENCES.as_str(),
                        scope: RouteScope::FILE.as_str(),
                        scope_key: file_scope_key,
                        provider: extraction.provider.as_str(),
                        run_id,
                        diagnostics_json: json!({
                            "kind": error.message(),
                            "error": error.to_string(),
                        }),
                    })
                    .await
                    .map_err(ExtractError::storage)?;
                Err(error)
            }
        }
    }

    async fn persist_references_after_scoped_route_started(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        run_id: i64,
        route: ScopedRoute<'_>,
        extraction: &ReferenceBatchExtraction,
        file_ids: &HashMap<String, i64>,
    ) -> ExtractResult<PersistenceSummary> {
        let mut summary = PersistenceSummary {
            workspace_id,
            run_id,
            files: 0,
            nodes: 0,
            edges: 0,
            reference_edges: 0,
            call_edges: 0,
            occurrences: 0,
            reference_occurrences: 0,
            call_occurrences: 0,
            evidence: 0,
            routes_complete: 0,
            stale_nodes_closed: 0,
            stale_edges_closed: 0,
        };

        for reference in &extraction.references {
            let reference_summary = self
                .persist_reference_after_scoped_route_started(
                    store,
                    workspace_id,
                    run_id,
                    reference,
                    file_ids,
                    route,
                )
                .await?;

            merge_summary(&mut summary, &reference_summary);
        }

        Ok(summary)
    }

    pub async fn persist_reference_after_route_started(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        run_id: i64,
        workspace_root_uri: &str,
        reference: &ExtractedReference,
        file_ids: &HashMap<String, i64>,
    ) -> ExtractResult<PersistenceSummary> {
        self.persist_reference_after_scoped_route_started(
            store,
            workspace_id,
            run_id,
            reference,
            file_ids,
            ScopedRoute::workspace(workspace_root_uri),
        )
        .await
    }

    async fn persist_reference_after_scoped_route_started(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        run_id: i64,
        reference: &ExtractedReference,
        file_ids: &HashMap<String, i64>,
        route: ScopedRoute<'_>,
    ) -> ExtractResult<PersistenceSummary> {
        let provider = reference.provider.as_str();
        self.require_node(
            store,
            workspace_id,
            provider,
            "textDocument/references",
            &reference.source_symbol_key,
        )
        .await?;
        self.require_node(
            store,
            workspace_id,
            provider,
            "textDocument/references",
            &reference.target_symbol_key,
        )
        .await?;

        let source_node_id = node_id(workspace_id, "rust", &reference.source_symbol_key);
        let target_node_id = node_id(workspace_id, "rust", &reference.target_symbol_key);
        let edge_id = store
            .upsert_edge(EdgeInput {
                workspace_id,
                src_node_id: &source_node_id,
                dst_node_id: &target_node_id,
                relation: "references",
                context: Some("symbol"),
                confidence: &reference.confidence,
                confidence_score: reference.confidence_score,
                weight: reference.occurrences.len() as f64,
                properties_json: json!({
                    "provider": reference.provider.as_str(),
                    "route": RouteName::RUST_REFERENCES.as_str(),
                    "source_resolution": reference.source_resolution,
                    "source_symbol_key": reference.source_symbol_key,
                    "target_symbol_key": reference.target_symbol_key,
                }),
                run_id: Some(run_id),
            })
            .await
            .map_err(ExtractError::storage)?;

        let mut summary = empty_summary(workspace_id, run_id);
        summary.edges += 1;
        summary.reference_edges += 1;

        for occurrence in &reference.occurrences {
            let file_id = *file_ids.get(&occurrence.file_uri).ok_or_else(|| {
                ExtractError::response_shape(
                    provider,
                    "textDocument/references",
                    format!(
                        "reference occurrence file {} was not in the current document-symbol batch",
                        occurrence.file_uri
                    ),
                )
            })?;
            let enclosing_node_id = occurrence
                .enclosing_symbol_key
                .as_ref()
                .map(|symbol_key| node_id(workspace_id, "rust", symbol_key));

            store
                .insert_occurrence(OccurrenceInput {
                    node_id: &target_node_id,
                    run_id,
                    file_id,
                    role: "reference",
                    range: occurrence.range,
                    enclosing_node_id: enclosing_node_id.as_deref(),
                    raw_json: Some(occurrence.raw_json.clone()),
                })
                .await
                .map_err(ExtractError::storage)?;
            store
                .insert_edge_evidence(EdgeEvidenceInput {
                    edge_id: &edge_id,
                    run_id,
                    provider: reference.provider.as_str(),
                    lsp_method: Some("textDocument/references"),
                    file_id: Some(file_id),
                    range: Some(occurrence.range),
                    raw_json: Some(json!({
                        "edge": reference.raw_json,
                        "occurrence": occurrence.raw_json,
                    })),
                })
                .await
                .map_err(ExtractError::storage)?;
            store
                .record_route_observation(RouteObservationInput {
                    workspace_id,
                    run_id,
                    route: RouteName::RUST_REFERENCES.as_str(),
                    scope: route.scope,
                    scope_key: route.scope_key,
                    provider,
                    entity_kind: "edge",
                    entity_id: &edge_id,
                    source_file_id: Some(file_id),
                    properties_json: json!({
                        "source": "textDocument/references",
                        "source_resolution": reference.source_resolution,
                    }),
                })
                .await
                .map_err(ExtractError::storage)?;

            summary.occurrences += 1;
            summary.reference_occurrences += 1;
            summary.evidence += 1;
        }

        Ok(summary)
    }

    pub async fn persist_call_batch_with_document_symbols(
        &self,
        store: &WriteHandle,
        workspace_root_uri: &str,
        extraction: &CallBatchExtraction,
    ) -> ExtractResult<PersistenceSummary> {
        let first_extraction =
            extraction
                .document_symbols
                .extractions
                .first()
                .ok_or_else(|| {
                    ExtractError::response_shape(
                        extraction.provider.as_str(),
                        "callHierarchy/outgoingCalls",
                        "call batch contained no document-symbol files",
                    )
                })?;
        let workspace_id = store
            .create_workspace(
                workspace_root_uri,
                first_extraction.source_file.language.workspace_kind(),
            )
            .await
            .map_err(ExtractError::storage)?;
        let run_id = store
            .start_run(
                workspace_id,
                extraction.provider.as_str(),
                extraction.provider_version.as_deref(),
                None,
            )
            .await
            .map_err(ExtractError::storage)?;

        let result = self
            .persist_call_batch_with_document_symbols_after_run_started(
                store,
                workspace_id,
                run_id,
                workspace_root_uri,
                extraction,
            )
            .await;

        match result {
            Ok(summary) => {
                store
                    .finish_run(run_id, "complete")
                    .await
                    .map_err(ExtractError::storage)?;
                Ok(summary)
            }
            Err(error) => {
                let finish_result = store.finish_run(run_id, "failed").await;
                if let Err(finish_error) = finish_result {
                    return Err(ExtractError::storage(finish_error));
                }
                Err(error)
            }
        }
    }

    pub async fn persist_call_batch(
        &self,
        store: &WriteHandle,
        workspace_root_uri: &str,
        extraction: &CallBatchExtraction,
    ) -> ExtractResult<PersistenceSummary> {
        let _first_extraction =
            extraction
                .document_symbols
                .extractions
                .first()
                .ok_or_else(|| {
                    ExtractError::response_shape(
                        extraction.provider.as_str(),
                        "callHierarchy/outgoingCalls",
                        "call batch contained no document-symbol files",
                    )
                })?;
        let workspace_id = self
            .existing_workspace_id(
                store,
                workspace_root_uri,
                extraction.provider.as_str(),
                "callHierarchy/outgoingCalls",
            )
            .await?;
        let run_id = store
            .start_run(
                workspace_id,
                extraction.provider.as_str(),
                extraction.provider_version.as_deref(),
                None,
            )
            .await
            .map_err(ExtractError::storage)?;

        let result = self
            .persist_call_batch_route_only_after_run_started(
                store,
                workspace_id,
                run_id,
                workspace_root_uri,
                extraction,
            )
            .await;

        match result {
            Ok(summary) => {
                store
                    .finish_run(run_id, "complete")
                    .await
                    .map_err(ExtractError::storage)?;
                Ok(summary)
            }
            Err(error) => {
                let finish_result = store.finish_run(run_id, "failed").await;
                if let Err(finish_error) = finish_result {
                    return Err(ExtractError::storage(finish_error));
                }
                Err(error)
            }
        }
    }

    pub async fn persist_call_file_batch(
        &self,
        store: &WriteHandle,
        workspace_root_uri: &str,
        extraction: &CallBatchExtraction,
    ) -> ExtractResult<PersistenceSummary> {
        let file_scope_key = single_file_scope_key(
            extraction.provider.as_str(),
            "callHierarchy/outgoingCalls",
            &extraction.document_symbols,
        )?;
        let workspace_id = self
            .existing_workspace_id(
                store,
                workspace_root_uri,
                extraction.provider.as_str(),
                "callHierarchy/outgoingCalls",
            )
            .await?;
        let run_id = store
            .start_run(
                workspace_id,
                extraction.provider.as_str(),
                extraction.provider_version.as_deref(),
                None,
            )
            .await
            .map_err(ExtractError::storage)?;

        let result = self
            .persist_call_file_batch_after_run_started(
                store,
                workspace_id,
                run_id,
                &file_scope_key,
                extraction,
            )
            .await;

        match result {
            Ok(summary) => {
                store
                    .finish_run(run_id, "complete")
                    .await
                    .map_err(ExtractError::storage)?;
                Ok(summary)
            }
            Err(error) => {
                let finish_result = store.finish_run(run_id, "failed").await;
                if let Err(finish_error) = finish_result {
                    return Err(ExtractError::storage(finish_error));
                }
                Err(error)
            }
        }
    }

    async fn persist_call_batch_route_only_after_run_started(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        run_id: i64,
        workspace_root_uri: &str,
        extraction: &CallBatchExtraction,
    ) -> ExtractResult<PersistenceSummary> {
        let file_ids = self
            .existing_document_symbol_file_ids(
                store,
                workspace_id,
                extraction.provider.as_str(),
                "callHierarchy/outgoingCalls",
                &extraction.document_symbols,
            )
            .await?;
        self.validate_call_nodes(store, workspace_id, extraction)
            .await?;

        store
            .start_route_status(RouteStatusStartInput {
                workspace_id,
                route: RouteName::RUST_CALLS.as_str(),
                scope: RouteScope::WORKSPACE.as_str(),
                scope_key: workspace_root_uri,
                file_id: None,
                provider: extraction.provider.as_str(),
                provider_version: extraction.provider_version.as_deref(),
                content_hash: Some(&extraction.workspace_fingerprint),
                run_id,
                diagnostics_json: json!({}),
            })
            .await
            .map_err(ExtractError::storage)?;

        let result = self
            .persist_calls_after_scoped_route_started(
                store,
                workspace_id,
                run_id,
                ScopedRoute::workspace(workspace_root_uri),
                extraction,
                &file_ids,
            )
            .await;

        match result {
            Ok(mut summary) => {
                store
                    .complete_route_status(RouteStatusCompleteInput {
                        workspace_id,
                        route: RouteName::RUST_CALLS.as_str(),
                        scope: RouteScope::WORKSPACE.as_str(),
                        scope_key: workspace_root_uri,
                        provider: extraction.provider.as_str(),
                        provider_version: extraction.provider_version.as_deref(),
                        content_hash: Some(&extraction.workspace_fingerprint),
                        run_id,
                        diagnostics_json: json!({
                            "callable_nodes": extraction.summary.callable_nodes,
                            "call_edges": summary.call_edges,
                            "call_occurrences": summary.call_occurrences,
                            "skipped_external_targets": extraction.summary.skipped_external_targets,
                            "skipped_unresolved_targets": extraction.summary.skipped_unresolved_targets,
                            "skipped_non_callable_prepare_items": extraction.summary.skipped_non_callable_prepare_items,
                        }),
                    })
                    .await
                    .map_err(ExtractError::storage)?;
                let stale_edges_closed = store
                    .close_stale_edges_for_route(CloseStaleRouteInput {
                        workspace_id,
                        run_id,
                        route: RouteName::RUST_CALLS.as_str(),
                        scope: RouteScope::WORKSPACE.as_str(),
                        scope_key: workspace_root_uri,
                        provider: extraction.provider.as_str(),
                    })
                    .await
                    .map_err(ExtractError::storage)?;

                summary.routes_complete = 1;
                summary.stale_edges_closed = stale_edges_closed as usize;

                Ok(summary)
            }
            Err(error) => {
                store
                    .fail_route_status(RouteStatusFailInput {
                        workspace_id,
                        route: RouteName::RUST_CALLS.as_str(),
                        scope: RouteScope::WORKSPACE.as_str(),
                        scope_key: workspace_root_uri,
                        provider: extraction.provider.as_str(),
                        run_id,
                        diagnostics_json: json!({
                            "kind": error.message(),
                            "error": error.to_string(),
                        }),
                    })
                    .await
                    .map_err(ExtractError::storage)?;
                Err(error)
            }
        }
    }

    async fn persist_call_batch_with_document_symbols_after_run_started(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        run_id: i64,
        workspace_root_uri: &str,
        extraction: &CallBatchExtraction,
    ) -> ExtractResult<PersistenceSummary> {
        let mut summary = self
            .persist_batch_after_run_started(
                store,
                workspace_id,
                run_id,
                &extraction.document_symbols,
            )
            .await?;
        let file_ids = self
            .document_symbol_file_ids(store, workspace_id, run_id, &extraction.document_symbols)
            .await?;

        store
            .start_route_status(RouteStatusStartInput {
                workspace_id,
                route: RouteName::RUST_CALLS.as_str(),
                scope: RouteScope::WORKSPACE.as_str(),
                scope_key: workspace_root_uri,
                file_id: None,
                provider: extraction.provider.as_str(),
                provider_version: extraction.provider_version.as_deref(),
                content_hash: Some(&extraction.workspace_fingerprint),
                run_id,
                diagnostics_json: json!({}),
            })
            .await
            .map_err(ExtractError::storage)?;

        let result = self
            .persist_calls_after_scoped_route_started(
                store,
                workspace_id,
                run_id,
                ScopedRoute::workspace(workspace_root_uri),
                extraction,
                &file_ids,
            )
            .await;

        match result {
            Ok(call_summary) => {
                store
                    .complete_route_status(RouteStatusCompleteInput {
                        workspace_id,
                        route: RouteName::RUST_CALLS.as_str(),
                        scope: RouteScope::WORKSPACE.as_str(),
                        scope_key: workspace_root_uri,
                        provider: extraction.provider.as_str(),
                        provider_version: extraction.provider_version.as_deref(),
                        content_hash: Some(&extraction.workspace_fingerprint),
                        run_id,
                        diagnostics_json: json!({
                            "callable_nodes": extraction.summary.callable_nodes,
                            "call_edges": call_summary.call_edges,
                            "call_occurrences": call_summary.call_occurrences,
                            "skipped_external_targets": extraction.summary.skipped_external_targets,
                            "skipped_unresolved_targets": extraction.summary.skipped_unresolved_targets,
                            "skipped_non_callable_prepare_items": extraction.summary.skipped_non_callable_prepare_items,
                        }),
                    })
                    .await
                    .map_err(ExtractError::storage)?;
                let stale_edges_closed = store
                    .close_stale_edges_for_route(CloseStaleRouteInput {
                        workspace_id,
                        run_id,
                        route: RouteName::RUST_CALLS.as_str(),
                        scope: RouteScope::WORKSPACE.as_str(),
                        scope_key: workspace_root_uri,
                        provider: extraction.provider.as_str(),
                    })
                    .await
                    .map_err(ExtractError::storage)?;

                summary.edges += call_summary.edges;
                summary.call_edges += call_summary.call_edges;
                summary.occurrences += call_summary.occurrences;
                summary.call_occurrences += call_summary.call_occurrences;
                summary.evidence += call_summary.evidence;
                summary.routes_complete += 1;
                summary.stale_edges_closed += stale_edges_closed as usize;

                Ok(summary)
            }
            Err(error) => {
                store
                    .fail_route_status(RouteStatusFailInput {
                        workspace_id,
                        route: RouteName::RUST_CALLS.as_str(),
                        scope: RouteScope::WORKSPACE.as_str(),
                        scope_key: workspace_root_uri,
                        provider: extraction.provider.as_str(),
                        run_id,
                        diagnostics_json: json!({
                            "kind": error.message(),
                            "error": error.to_string(),
                        }),
                    })
                    .await
                    .map_err(ExtractError::storage)?;
                Err(error)
            }
        }
    }

    async fn persist_call_file_batch_after_run_started(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        run_id: i64,
        file_scope_key: &str,
        extraction: &CallBatchExtraction,
    ) -> ExtractResult<PersistenceSummary> {
        let file_ids = self
            .existing_document_symbol_file_ids(
                store,
                workspace_id,
                extraction.provider.as_str(),
                "callHierarchy/outgoingCalls",
                &extraction.document_symbols,
            )
            .await?;
        self.validate_call_nodes(store, workspace_id, extraction)
            .await?;
        let file_id = *file_ids.get(file_scope_key).ok_or_else(|| {
            ExtractError::response_shape(
                extraction.provider.as_str(),
                "callHierarchy/outgoingCalls",
                format!("source file {file_scope_key} is missing from the database"),
            )
        })?;
        let file_content_hash = single_file_content_hash(
            extraction.provider.as_str(),
            "callHierarchy/outgoingCalls",
            &extraction.document_symbols,
        )?;

        store
            .start_route_status(RouteStatusStartInput {
                workspace_id,
                route: RouteName::RUST_CALLS.as_str(),
                scope: RouteScope::FILE.as_str(),
                scope_key: file_scope_key,
                file_id: Some(file_id),
                provider: extraction.provider.as_str(),
                provider_version: extraction.provider_version.as_deref(),
                content_hash: file_content_hash.as_deref(),
                run_id,
                diagnostics_json: json!({}),
            })
            .await
            .map_err(ExtractError::storage)?;

        let result = self
            .persist_calls_after_scoped_route_started(
                store,
                workspace_id,
                run_id,
                ScopedRoute::file(file_scope_key),
                extraction,
                &file_ids,
            )
            .await;

        match result {
            Ok(mut summary) => {
                store
                    .complete_route_status(RouteStatusCompleteInput {
                        workspace_id,
                        route: RouteName::RUST_CALLS.as_str(),
                        scope: RouteScope::FILE.as_str(),
                        scope_key: file_scope_key,
                        provider: extraction.provider.as_str(),
                        provider_version: extraction.provider_version.as_deref(),
                        content_hash: file_content_hash.as_deref(),
                        run_id,
                        diagnostics_json: json!({
                            "callable_nodes": extraction.summary.callable_nodes,
                            "call_edges": summary.call_edges,
                            "call_occurrences": summary.call_occurrences,
                            "skipped_external_targets": extraction.summary.skipped_external_targets,
                            "skipped_unresolved_targets": extraction.summary.skipped_unresolved_targets,
                            "skipped_non_callable_prepare_items": extraction.summary.skipped_non_callable_prepare_items,
                        }),
                    })
                    .await
                    .map_err(ExtractError::storage)?;
                let stale_edges_closed = store
                    .close_stale_edges_for_route(CloseStaleRouteInput {
                        workspace_id,
                        run_id,
                        route: RouteName::RUST_CALLS.as_str(),
                        scope: RouteScope::FILE.as_str(),
                        scope_key: file_scope_key,
                        provider: extraction.provider.as_str(),
                    })
                    .await
                    .map_err(ExtractError::storage)?;

                summary.routes_complete = 1;
                summary.stale_edges_closed = stale_edges_closed as usize;

                Ok(summary)
            }
            Err(error) => {
                store
                    .fail_route_status(RouteStatusFailInput {
                        workspace_id,
                        route: RouteName::RUST_CALLS.as_str(),
                        scope: RouteScope::FILE.as_str(),
                        scope_key: file_scope_key,
                        provider: extraction.provider.as_str(),
                        run_id,
                        diagnostics_json: json!({
                            "kind": error.message(),
                            "error": error.to_string(),
                        }),
                    })
                    .await
                    .map_err(ExtractError::storage)?;
                Err(error)
            }
        }
    }

    async fn persist_calls_after_scoped_route_started(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        run_id: i64,
        route: ScopedRoute<'_>,
        extraction: &CallBatchExtraction,
        file_ids: &HashMap<String, i64>,
    ) -> ExtractResult<PersistenceSummary> {
        let mut summary = PersistenceSummary {
            workspace_id,
            run_id,
            files: 0,
            nodes: 0,
            edges: 0,
            reference_edges: 0,
            call_edges: 0,
            occurrences: 0,
            reference_occurrences: 0,
            call_occurrences: 0,
            evidence: 0,
            routes_complete: 0,
            stale_nodes_closed: 0,
            stale_edges_closed: 0,
        };

        for call in &extraction.calls {
            let call_summary = self
                .persist_call_after_scoped_route_started(
                    store,
                    workspace_id,
                    run_id,
                    call,
                    file_ids,
                    route,
                )
                .await?;

            merge_summary(&mut summary, &call_summary);
        }

        Ok(summary)
    }

    pub async fn persist_call_after_route_started(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        run_id: i64,
        workspace_root_uri: &str,
        call: &ExtractedCall,
        file_ids: &HashMap<String, i64>,
    ) -> ExtractResult<PersistenceSummary> {
        self.persist_call_after_scoped_route_started(
            store,
            workspace_id,
            run_id,
            call,
            file_ids,
            ScopedRoute::workspace(workspace_root_uri),
        )
        .await
    }

    async fn persist_call_after_scoped_route_started(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        run_id: i64,
        call: &ExtractedCall,
        file_ids: &HashMap<String, i64>,
        route: ScopedRoute<'_>,
    ) -> ExtractResult<PersistenceSummary> {
        let provider = call.provider.as_str();
        self.require_node(
            store,
            workspace_id,
            provider,
            "callHierarchy/outgoingCalls",
            &call.caller_symbol_key,
        )
        .await?;
        self.require_node(
            store,
            workspace_id,
            provider,
            "callHierarchy/outgoingCalls",
            &call.callee_symbol_key,
        )
        .await?;

        let caller_node_id = node_id(workspace_id, "rust", &call.caller_symbol_key);
        let callee_node_id = node_id(workspace_id, "rust", &call.callee_symbol_key);
        let edge_id = store
            .upsert_edge(EdgeInput {
                workspace_id,
                src_node_id: &caller_node_id,
                dst_node_id: &callee_node_id,
                relation: "calls",
                context: Some(&call.context),
                confidence: &call.confidence,
                confidence_score: call.confidence_score,
                weight: call.occurrences.len() as f64,
                properties_json: json!({
                    "provider": call.provider.as_str(),
                    "route": RouteName::RUST_CALLS.as_str(),
                    "caller_symbol_key": call.caller_symbol_key,
                    "callee_symbol_key": call.callee_symbol_key,
                }),
                run_id: Some(run_id),
            })
            .await
            .map_err(ExtractError::storage)?;

        let mut summary = empty_summary(workspace_id, run_id);
        summary.edges += 1;
        summary.call_edges += 1;

        for occurrence in &call.occurrences {
            let file_id = *file_ids.get(&occurrence.file_uri).ok_or_else(|| {
                ExtractError::response_shape(
                    provider,
                    "callHierarchy/outgoingCalls",
                    format!(
                        "call occurrence file {} was not in the current document-symbol batch",
                        occurrence.file_uri
                    ),
                )
            })?;
            let enclosing_node_id = node_id(workspace_id, "rust", &occurrence.enclosing_symbol_key);

            store
                .insert_occurrence(OccurrenceInput {
                    node_id: &callee_node_id,
                    run_id,
                    file_id,
                    role: "call",
                    range: occurrence.range,
                    enclosing_node_id: Some(&enclosing_node_id),
                    raw_json: Some(occurrence.raw_json.clone()),
                })
                .await
                .map_err(ExtractError::storage)?;
            store
                .insert_edge_evidence(EdgeEvidenceInput {
                    edge_id: &edge_id,
                    run_id,
                    provider: call.provider.as_str(),
                    lsp_method: Some("callHierarchy/outgoingCalls"),
                    file_id: Some(file_id),
                    range: Some(occurrence.range),
                    raw_json: Some(json!({
                        "edge": call.raw_json,
                        "occurrence": occurrence.raw_json,
                    })),
                })
                .await
                .map_err(ExtractError::storage)?;
            store
                .record_route_observation(RouteObservationInput {
                    workspace_id,
                    run_id,
                    route: RouteName::RUST_CALLS.as_str(),
                    scope: route.scope,
                    scope_key: route.scope_key,
                    provider,
                    entity_kind: "edge",
                    entity_id: &edge_id,
                    source_file_id: Some(file_id),
                    properties_json: json!({
                        "source": "callHierarchy/outgoingCalls",
                        "context": call.context,
                    }),
                })
                .await
                .map_err(ExtractError::storage)?;

            summary.occurrences += 1;
            summary.call_occurrences += 1;
            summary.evidence += 1;
        }

        Ok(summary)
    }

    async fn document_symbol_file_ids(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        run_id: i64,
        extraction: &DocumentSymbolBatchExtraction,
    ) -> ExtractResult<HashMap<String, i64>> {
        let mut file_ids = HashMap::new();
        for file_extraction in &extraction.extractions {
            let file_id = store
                .upsert_file(FileInput {
                    workspace_id,
                    uri: &file_extraction.source_file.uri,
                    path: &file_extraction.source_file.relative_path,
                    language: file_extraction.source_file.language.as_store_str(),
                    content_hash: file_extraction.source_file.content_hash.as_deref(),
                    last_seen_run_id: Some(run_id),
                    properties_json: json!({
                        "provider": file_extraction.provider.as_str(),
                        "language": file_extraction.source_file.language.as_store_str(),
                        "raw_metadata": file_extraction.raw_metadata,
                    }),
                })
                .await
                .map_err(ExtractError::storage)?;
            file_ids.insert(file_extraction.source_file.uri.clone(), file_id);
        }

        Ok(file_ids)
    }

    async fn existing_workspace_id(
        &self,
        store: &WriteHandle,
        workspace_root_uri: &str,
        provider: &str,
        method: &str,
    ) -> ExtractResult<i64> {
        store
            .workspace_id(workspace_root_uri)
            .await
            .map_err(ExtractError::storage)?
            .ok_or_else(|| {
                ExtractError::response_shape(
                    provider,
                    method,
                    format!(
                        "workspace {workspace_root_uri} is missing; run rust-workspace-document-symbols first, run rust-file --symbols for one file, or use rust-workspace-all"
                    ),
                )
            })
    }

    async fn existing_document_symbol_file_ids(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        provider: &str,
        method: &str,
        extraction: &DocumentSymbolBatchExtraction,
    ) -> ExtractResult<HashMap<String, i64>> {
        let mut file_ids = HashMap::new();
        for file_extraction in &extraction.extractions {
            let file_id = store
                .file_id(workspace_id, &file_extraction.source_file.uri)
                .await
                .map_err(ExtractError::storage)?
                .ok_or_else(|| {
                    ExtractError::response_shape(
                        provider,
                        method,
                        format!(
                            "source file {} is missing from the database; run rust-workspace-document-symbols first, run rust-file --symbols for one file, or use rust-workspace-all",
                            file_extraction.source_file.uri
                        ),
                    )
                })?;
            file_ids.insert(file_extraction.source_file.uri.clone(), file_id);
        }

        Ok(file_ids)
    }

    async fn validate_reference_nodes(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        extraction: &ReferenceBatchExtraction,
    ) -> ExtractResult<()> {
        for reference in &extraction.references {
            self.require_node(
                store,
                workspace_id,
                extraction.provider.as_str(),
                "textDocument/references",
                &reference.source_symbol_key,
            )
            .await?;
            self.require_node(
                store,
                workspace_id,
                extraction.provider.as_str(),
                "textDocument/references",
                &reference.target_symbol_key,
            )
            .await?;
        }

        Ok(())
    }

    async fn validate_call_nodes(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        extraction: &CallBatchExtraction,
    ) -> ExtractResult<()> {
        for call in &extraction.calls {
            self.require_node(
                store,
                workspace_id,
                extraction.provider.as_str(),
                "callHierarchy/outgoingCalls",
                &call.caller_symbol_key,
            )
            .await?;
            self.require_node(
                store,
                workspace_id,
                extraction.provider.as_str(),
                "callHierarchy/outgoingCalls",
                &call.callee_symbol_key,
            )
            .await?;
        }

        Ok(())
    }

    async fn require_node(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        provider: &str,
        method: &str,
        symbol_key: &str,
    ) -> ExtractResult<()> {
        let id = node_id(workspace_id, "rust", symbol_key);
        if store
            .node_exists(&id)
            .await
            .map_err(ExtractError::storage)?
        {
            return Ok(());
        }

        Err(ExtractError::response_shape(
            provider,
            method,
            format!(
                "symbol node {symbol_key} is missing from the database; run rust-workspace-document-symbols first, run rust-file --symbols for one file, or use rust-workspace-all"
            ),
        ))
    }

    async fn persist_batch_after_run_started(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        run_id: i64,
        extraction: &DocumentSymbolBatchExtraction,
    ) -> ExtractResult<PersistenceSummary> {
        let mut summary = PersistenceSummary {
            workspace_id,
            run_id,
            files: 0,
            nodes: 0,
            edges: 0,
            reference_edges: 0,
            call_edges: 0,
            occurrences: 0,
            reference_occurrences: 0,
            call_occurrences: 0,
            evidence: 0,
            routes_complete: 0,
            stale_nodes_closed: 0,
            stale_edges_closed: 0,
        };

        for file_extraction in &extraction.extractions {
            let file_summary = self
                .persist_after_run_started(store, workspace_id, run_id, file_extraction)
                .await?;
            summary.files += file_summary.files;
            summary.nodes += file_summary.nodes;
            summary.edges += file_summary.edges;
            summary.reference_edges += file_summary.reference_edges;
            summary.call_edges += file_summary.call_edges;
            summary.occurrences += file_summary.occurrences;
            summary.reference_occurrences += file_summary.reference_occurrences;
            summary.call_occurrences += file_summary.call_occurrences;
            summary.evidence += file_summary.evidence;
            summary.routes_complete += file_summary.routes_complete;
            summary.stale_nodes_closed += file_summary.stale_nodes_closed;
            summary.stale_edges_closed += file_summary.stale_edges_closed;
        }

        Ok(summary)
    }

    async fn persist_after_run_started(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        run_id: i64,
        extraction: &DocumentSymbolExtraction,
    ) -> ExtractResult<PersistenceSummary> {
        let file_id = store
            .upsert_file(FileInput {
                workspace_id,
                uri: &extraction.source_file.uri,
                path: &extraction.source_file.relative_path,
                language: extraction.source_file.language.as_store_str(),
                content_hash: extraction.source_file.content_hash.as_deref(),
                last_seen_run_id: Some(run_id),
                properties_json: json!({
                    "provider": extraction.provider.as_str(),
                    "language": extraction.source_file.language.as_store_str(),
                    "raw_metadata": extraction.raw_metadata,
                }),
            })
            .await
            .map_err(ExtractError::storage)?;

        store
            .start_route_status(RouteStatusStartInput {
                workspace_id,
                route: RouteName::RUST_DOCUMENT_SYMBOLS.as_str(),
                scope: RouteScope::FILE.as_str(),
                scope_key: &extraction.source_file.uri,
                file_id: Some(file_id),
                provider: extraction.provider.as_str(),
                provider_version: extraction.provider_version.as_deref(),
                content_hash: extraction.source_file.content_hash.as_deref(),
                run_id,
                diagnostics_json: json!({}),
            })
            .await
            .map_err(ExtractError::storage)?;

        let result = self
            .persist_file_after_route_started(store, workspace_id, run_id, file_id, extraction)
            .await;

        match result {
            Ok(mut summary) => {
                store
                    .complete_route_status(RouteStatusCompleteInput {
                        workspace_id,
                        route: RouteName::RUST_DOCUMENT_SYMBOLS.as_str(),
                        scope: RouteScope::FILE.as_str(),
                        scope_key: &extraction.source_file.uri,
                        provider: extraction.provider.as_str(),
                        provider_version: extraction.provider_version.as_deref(),
                        content_hash: extraction.source_file.content_hash.as_deref(),
                        run_id,
                        diagnostics_json: json!({
                            "files": summary.files,
                            "nodes": summary.nodes,
                            "contains_edges": summary.edges,
                            "occurrences": summary.occurrences,
                            "evidence": summary.evidence,
                        }),
                    })
                    .await
                    .map_err(ExtractError::storage)?;

                let stale_nodes_closed = store
                    .close_stale_nodes_for_route(CloseStaleRouteInput {
                        workspace_id,
                        run_id,
                        route: RouteName::RUST_DOCUMENT_SYMBOLS.as_str(),
                        scope: RouteScope::FILE.as_str(),
                        scope_key: &extraction.source_file.uri,
                        provider: extraction.provider.as_str(),
                    })
                    .await
                    .map_err(ExtractError::storage)?;
                let stale_edges_closed = store
                    .close_stale_edges_for_route(CloseStaleRouteInput {
                        workspace_id,
                        run_id,
                        route: RouteName::RUST_DOCUMENT_SYMBOLS.as_str(),
                        scope: RouteScope::FILE.as_str(),
                        scope_key: &extraction.source_file.uri,
                        provider: extraction.provider.as_str(),
                    })
                    .await
                    .map_err(ExtractError::storage)?;

                summary.routes_complete = 1;
                summary.stale_nodes_closed = stale_nodes_closed as usize;
                summary.stale_edges_closed = stale_edges_closed as usize;
                Ok(summary)
            }
            Err(error) => {
                store
                    .fail_route_status(RouteStatusFailInput {
                        workspace_id,
                        route: RouteName::RUST_DOCUMENT_SYMBOLS.as_str(),
                        scope: RouteScope::FILE.as_str(),
                        scope_key: &extraction.source_file.uri,
                        provider: extraction.provider.as_str(),
                        run_id,
                        diagnostics_json: json!({
                            "kind": error.message(),
                            "error": error.to_string(),
                        }),
                    })
                    .await
                    .map_err(ExtractError::storage)?;
                Err(error)
            }
        }
    }

    async fn persist_file_after_route_started(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        run_id: i64,
        file_id: i64,
        extraction: &DocumentSymbolExtraction,
    ) -> ExtractResult<PersistenceSummary> {
        let mut node_ids = HashMap::new();
        let file_node_id = self
            .upsert_file_node(store, workspace_id, run_id, file_id, extraction)
            .await?;
        self.record_document_symbol_node_observation(
            store,
            workspace_id,
            run_id,
            file_id,
            extraction,
            &file_node_id,
        )
        .await?;
        node_ids.insert(extraction.source_file.file_symbol_key.clone(), file_node_id);

        for symbol in &extraction.symbols {
            let container_node_id = if let Some(parent_symbol_key) = &symbol.parent_symbol_key {
                Some(
                    node_ids
                        .get(parent_symbol_key)
                        .ok_or_else(|| {
                            ExtractError::response_shape(
                                extraction.provider.as_str(),
                                "textDocument/documentSymbol",
                                format!(
                                    "symbol {} references missing parent {}",
                                    symbol.symbol_key, parent_symbol_key
                                ),
                            )
                        })?
                        .clone(),
                )
            } else {
                None
            };

            let node_id = store
                .upsert_node(NodeInput {
                    workspace_id,
                    language: symbol.language.as_store_str(),
                    kind: &symbol.kind,
                    name: &symbol.name,
                    qualified_name: symbol.qualified_name.as_deref(),
                    display_name: Some(&symbol.name),
                    symbol_key: &symbol.symbol_key,
                    file_id: Some(file_id),
                    range: Some(symbol.range),
                    selection_range: Some(symbol.selection_range),
                    container_node_id: container_node_id.as_deref(),
                    properties_json: symbol_properties_json(symbol),
                    run_id: Some(run_id),
                })
                .await
                .map_err(ExtractError::storage)?;

            self.record_document_symbol_node_observation(
                store,
                workspace_id,
                run_id,
                file_id,
                extraction,
                &node_id,
            )
            .await?;

            store
                .insert_occurrence(OccurrenceInput {
                    node_id: &node_id,
                    run_id,
                    file_id,
                    role: "definition",
                    range: symbol.selection_range,
                    enclosing_node_id: container_node_id.as_deref(),
                    raw_json: Some(symbol.raw_json.clone()),
                })
                .await
                .map_err(ExtractError::storage)?;

            node_ids.insert(symbol.symbol_key.clone(), node_id);
        }

        for relation in &extraction.relations {
            let src_node_id = node_ids
                .get(&relation.source_symbol_key)
                .ok_or_else(|| {
                    ExtractError::response_shape(
                        extraction.provider.as_str(),
                        "textDocument/documentSymbol",
                        format!(
                            "relation source {} was not persisted",
                            relation.source_symbol_key
                        ),
                    )
                })?
                .clone();
            let dst_node_id = node_ids
                .get(&relation.target_symbol_key)
                .ok_or_else(|| {
                    ExtractError::response_shape(
                        extraction.provider.as_str(),
                        "textDocument/documentSymbol",
                        format!(
                            "relation target {} was not persisted",
                            relation.target_symbol_key
                        ),
                    )
                })?
                .clone();

            let edge_id = store
                .upsert_edge(EdgeInput {
                    workspace_id,
                    src_node_id: &src_node_id,
                    dst_node_id: &dst_node_id,
                    relation: &relation.relation,
                    context: None,
                    confidence: &relation.confidence,
                    confidence_score: relation.confidence_score,
                    weight: 1.0,
                    properties_json: json!({
                        "provider": relation.provider.as_str(),
                        "source": "textDocument/documentSymbol",
                    }),
                    run_id: Some(run_id),
                })
                .await
                .map_err(ExtractError::storage)?;

            self.record_document_symbol_edge_observation(
                store,
                workspace_id,
                run_id,
                file_id,
                extraction,
                &edge_id,
            )
            .await?;

            store
                .insert_edge_evidence(EdgeEvidenceInput {
                    edge_id: &edge_id,
                    run_id,
                    provider: relation.provider.as_str(),
                    lsp_method: Some("textDocument/documentSymbol"),
                    file_id: Some(file_id),
                    range: relation.range,
                    raw_json: Some(relation.raw_json.clone()),
                })
                .await
                .map_err(ExtractError::storage)?;
        }

        Ok(PersistenceSummary {
            workspace_id,
            run_id,
            files: 1,
            nodes: node_ids.len(),
            edges: extraction.relations.len(),
            reference_edges: 0,
            call_edges: 0,
            occurrences: extraction.symbols.len(),
            reference_occurrences: 0,
            call_occurrences: 0,
            evidence: extraction.relations.len(),
            routes_complete: 0,
            stale_nodes_closed: 0,
            stale_edges_closed: 0,
        })
    }

    async fn upsert_file_node(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        run_id: i64,
        file_id: i64,
        extraction: &DocumentSymbolExtraction,
    ) -> ExtractResult<String> {
        let file_name = basename_from_relative_path(&extraction.source_file.relative_path);

        store
            .upsert_node(NodeInput {
                workspace_id,
                language: extraction.source_file.language.as_store_str(),
                kind: "file",
                name: &file_name,
                qualified_name: Some(&extraction.source_file.relative_path),
                display_name: Some(&file_name),
                symbol_key: &extraction.source_file.file_symbol_key,
                file_id: Some(file_id),
                range: None,
                selection_range: None,
                container_node_id: None,
                properties_json: json!({
                    "provider": extraction.provider.as_str(),
                    "language": extraction.source_file.language.as_store_str(),
                    "uri": extraction.source_file.uri,
                }),
                run_id: Some(run_id),
            })
            .await
            .map_err(ExtractError::storage)
    }

    async fn record_document_symbol_node_observation(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        run_id: i64,
        file_id: i64,
        extraction: &DocumentSymbolExtraction,
        node_id: &str,
    ) -> ExtractResult<()> {
        store
            .record_route_observation(RouteObservationInput {
                workspace_id,
                run_id,
                route: RouteName::RUST_DOCUMENT_SYMBOLS.as_str(),
                scope: RouteScope::FILE.as_str(),
                scope_key: &extraction.source_file.uri,
                provider: extraction.provider.as_str(),
                entity_kind: "node",
                entity_id: node_id,
                source_file_id: Some(file_id),
                properties_json: json!({
                    "source": "textDocument/documentSymbol",
                }),
            })
            .await
            .map_err(ExtractError::storage)
    }

    async fn record_document_symbol_edge_observation(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        run_id: i64,
        file_id: i64,
        extraction: &DocumentSymbolExtraction,
        edge_id: &str,
    ) -> ExtractResult<()> {
        store
            .record_route_observation(RouteObservationInput {
                workspace_id,
                run_id,
                route: RouteName::RUST_DOCUMENT_SYMBOLS.as_str(),
                scope: RouteScope::FILE.as_str(),
                scope_key: &extraction.source_file.uri,
                provider: extraction.provider.as_str(),
                entity_kind: "edge",
                entity_id: edge_id,
                source_file_id: Some(file_id),
                properties_json: json!({
                    "relation": "contains",
                    "source": "textDocument/documentSymbol",
                }),
            })
            .await
            .map_err(ExtractError::storage)
    }
}

fn single_file_scope_key(
    provider: &str,
    method: &str,
    extraction: &DocumentSymbolBatchExtraction,
) -> ExtractResult<String> {
    match extraction.extractions.as_slice() {
        [file_extraction] => Ok(file_extraction.source_file.uri.clone()),
        [] => Err(ExtractError::response_shape(
            provider,
            method,
            "single-file relation extraction contained no document-symbol files",
        )),
        _ => Err(ExtractError::response_shape(
            provider,
            method,
            "single-file relation extraction contained more than one document-symbol file",
        )),
    }
}

fn single_file_content_hash(
    provider: &str,
    method: &str,
    extraction: &DocumentSymbolBatchExtraction,
) -> ExtractResult<Option<String>> {
    match extraction.extractions.as_slice() {
        [file_extraction] => Ok(file_extraction.source_file.content_hash.clone()),
        [] => Err(ExtractError::response_shape(
            provider,
            method,
            "single-file relation extraction contained no document-symbol files",
        )),
        _ => Err(ExtractError::response_shape(
            provider,
            method,
            "single-file relation extraction contained more than one document-symbol file",
        )),
    }
}

fn symbol_properties_json(symbol: &crate::model::ExtractedSymbol) -> Value {
    json!({
        "provider": symbol.provider.as_str(),
        "language": symbol.language.as_store_str(),
        "lsp_kind": symbol.raw_json.get("lsp_kind").cloned(),
        "detail": symbol.detail.as_deref(),
        "raw": symbol.raw_json,
    })
}

fn empty_summary(workspace_id: i64, run_id: i64) -> PersistenceSummary {
    PersistenceSummary {
        workspace_id,
        run_id,
        files: 0,
        nodes: 0,
        edges: 0,
        reference_edges: 0,
        call_edges: 0,
        occurrences: 0,
        reference_occurrences: 0,
        call_occurrences: 0,
        evidence: 0,
        routes_complete: 0,
        stale_nodes_closed: 0,
        stale_edges_closed: 0,
    }
}

fn merge_summary(target: &mut PersistenceSummary, source: &PersistenceSummary) {
    target.files += source.files;
    target.nodes += source.nodes;
    target.edges += source.edges;
    target.reference_edges += source.reference_edges;
    target.call_edges += source.call_edges;
    target.occurrences += source.occurrences;
    target.reference_occurrences += source.reference_occurrences;
    target.call_occurrences += source.call_occurrences;
    target.evidence += source.evidence;
    target.routes_complete += source.routes_complete;
    target.stale_nodes_closed += source.stale_nodes_closed;
    target.stale_edges_closed += source.stale_edges_closed;
}
