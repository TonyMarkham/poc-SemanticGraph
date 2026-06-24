use crate::{
    ExtractError, ExtractResult,
    cli::symbol_key_belongs_to_file,
    document_symbols::paths::basename_from_relative_path,
    model::{
        CallBatchExtraction, CallRouteSummary, DocumentSymbolBatchExtraction,
        DocumentSymbolExtraction, ExtractedCall, ExtractedReference, GraphLanguage, ProviderId,
        ReferenceBatchExtraction, ReferenceRouteSummary, RouteName, RouteScope,
    },
    persist::{PersistenceRun, PersistenceSummary, ScopedRoute},
};

use semantic_graph_db_manager::{
    CloseStaleFileInput, CloseStaleRouteInput, DocumentSymbolWriteBatchCloseStaleRouteInput,
    DocumentSymbolWriteBatchEdgeEvidenceInput, DocumentSymbolWriteBatchFileInput,
    DocumentSymbolWriteBatchInput, DocumentSymbolWriteBatchNodeInput,
    DocumentSymbolWriteBatchObservationInput, DocumentSymbolWriteBatchOccurrenceInput,
    DocumentSymbolWriteBatchRouteStatusCompleteInput,
    DocumentSymbolWriteBatchRouteStatusStartInput, EdgeEvidenceInput, EdgeInput, FileInput,
    NodeInput, OccurrenceInput, RouteObservationInput, RouteStatusCompleteInput,
    RouteStatusFailInput, RouteStatusStartInput, RouteWriteBatchEdgeEvidenceInput,
    RouteWriteBatchEdgeInput, RouteWriteBatchInput, RouteWriteBatchObservationInput,
    RouteWriteBatchOccurrenceInput, WriteHandle, edge_id, node_id,
};

use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

pub struct ExtractionPersister;

impl ExtractionPersister {
    pub async fn mark_deleted_rust_file_stale(
        &self,
        store: &WriteHandle,
        workspace_root_uri: &str,
        file_uri: &str,
    ) -> ExtractResult<PersistenceSummary> {
        self.mark_deleted_file_stale(
            store,
            workspace_root_uri,
            file_uri,
            GraphLanguage::Rust,
            ProviderId::rust_analyzer(),
            "rust-file-deleted",
        )
        .await
    }

    pub async fn mark_deleted_file_stale(
        &self,
        store: &WriteHandle,
        workspace_root_uri: &str,
        file_uri: &str,
        language: GraphLanguage,
        provider: ProviderId,
        source: &'static str,
    ) -> ExtractResult<PersistenceSummary> {
        let workspace_id = store
            .create_workspace(workspace_root_uri, language.workspace_kind())
            .await
            .map_err(ExtractError::storage)?;
        let run_id = store
            .start_run(workspace_id, provider.as_str(), None, None)
            .await
            .map_err(ExtractError::storage)?;

        let result = self
            .mark_deleted_file_stale_after_run_started(
                store,
                PersistenceRun {
                    workspace_id,
                    run_id,
                },
                provider,
                file_uri,
                language,
                source,
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

    async fn mark_deleted_file_stale_after_run_started(
        &self,
        store: &WriteHandle,
        run: PersistenceRun,
        provider: ProviderId,
        file_uri: &str,
        language: GraphLanguage,
        source: &'static str,
    ) -> ExtractResult<PersistenceSummary> {
        let file_id = store
            .file_id(run.workspace_id, file_uri)
            .await
            .map_err(ExtractError::storage)?;
        let mut summary = empty_summary(run.workspace_id, run.run_id);

        for route in [
            RouteName::document_symbols_for_language(language),
            RouteName::references_for_language(language),
            RouteName::calls_for_language(language),
        ] {
            store
                .start_route_status(RouteStatusStartInput {
                    workspace_id: run.workspace_id,
                    route: route.as_str(),
                    scope: RouteScope::FILE.as_str(),
                    scope_key: file_uri,
                    file_id,
                    provider: provider.as_str(),
                    provider_version: None,
                    content_hash: None,
                    run_id: run.run_id,
                    diagnostics_json: json!({
                        "file_deleted": true,
                        "source": source,
                    }),
                })
                .await
                .map_err(ExtractError::storage)?;

            store
                .complete_route_status(RouteStatusCompleteInput {
                    workspace_id: run.workspace_id,
                    route: route.as_str(),
                    scope: RouteScope::FILE.as_str(),
                    scope_key: file_uri,
                    provider: provider.as_str(),
                    provider_version: None,
                    content_hash: None,
                    run_id: run.run_id,
                    diagnostics_json: json!({
                        "file_deleted": true,
                        "observations": 0,
                        "source": source,
                    }),
                })
                .await
                .map_err(ExtractError::storage)?;
            summary.routes_complete += 1;
        }

        let stale_summary = store
            .close_stale_file(CloseStaleFileInput {
                workspace_id: run.workspace_id,
                run_id: run.run_id,
                file_uri,
            })
            .await
            .map_err(ExtractError::storage)?;
        summary.stale_nodes_closed = stale_summary.stale_nodes_closed as usize;
        summary.stale_edges_closed = stale_summary.stale_edges_closed as usize;

        Ok(summary)
    }

    pub async fn persist_document_symbols(
        &self,
        store: &WriteHandle,
        workspace_root_uri: &str,
        extraction: &DocumentSymbolExtraction,
    ) -> ExtractResult<PersistenceSummary> {
        let workspace_id = store
            .create_workspace(workspace_root_uri, extraction.language.workspace_kind())
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
        if extraction.extractions.is_empty() {
            return Err(ExtractError::response_shape(
                extraction.provider.as_str(),
                "textDocument/documentSymbol",
                "document symbol batch contained no files",
            ));
        }
        let workspace_id = store
            .create_workspace(workspace_root_uri, extraction.language.workspace_kind())
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

    pub async fn persist_document_symbol_batch_with_write_batch(
        &self,
        store: &WriteHandle,
        workspace_root_uri: &str,
        extraction: &DocumentSymbolBatchExtraction,
        close_stale: bool,
    ) -> ExtractResult<PersistenceSummary> {
        if extraction.extractions.is_empty() {
            return Err(ExtractError::response_shape(
                extraction.provider.as_str(),
                "textDocument/documentSymbol",
                "document symbol batch contained no files",
            ));
        }
        let workspace_id = store
            .create_workspace(workspace_root_uri, extraction.language.workspace_kind())
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
            .persist_batch_with_write_batch_after_run_started(
                store,
                workspace_id,
                run_id,
                extraction,
                close_stale,
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

    pub async fn persist_reference_batch_with_document_symbols(
        &self,
        store: &WriteHandle,
        workspace_root_uri: &str,
        extraction: &ReferenceBatchExtraction,
    ) -> ExtractResult<PersistenceSummary> {
        if extraction.document_symbols.extractions.is_empty() {
            return Err(ExtractError::response_shape(
                extraction.provider.as_str(),
                "textDocument/references",
                "reference batch contained no document-symbol files",
            ));
        }
        let workspace_id = store
            .create_workspace(
                workspace_root_uri,
                extraction.document_symbols.language.workspace_kind(),
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
        let language = document_symbol_batch_language(
            extraction.provider.as_str(),
            "textDocument/references",
            &extraction.document_symbols,
        )?;
        let workspace_id = self
            .existing_workspace_id(
                store,
                workspace_root_uri,
                language,
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

    pub async fn persist_reference_batch_with_route_write_batch(
        &self,
        store: &WriteHandle,
        workspace_root_uri: &str,
        extraction: &ReferenceBatchExtraction,
    ) -> ExtractResult<PersistenceSummary> {
        let language = document_symbol_batch_language(
            extraction.provider.as_str(),
            "textDocument/references",
            &extraction.document_symbols,
        )?;
        let workspace_id = self
            .existing_workspace_id(
                store,
                workspace_root_uri,
                language,
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
            .persist_reference_batch_with_route_write_batch_after_run_started(
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
        self.persist_reference_file_batch_for_file(
            store,
            workspace_root_uri,
            &file_scope_key,
            extraction,
        )
        .await
    }

    pub async fn persist_reference_file_batch_for_file(
        &self,
        store: &WriteHandle,
        workspace_root_uri: &str,
        file_scope_key: &str,
        extraction: &ReferenceBatchExtraction,
    ) -> ExtractResult<PersistenceSummary> {
        let language = document_symbol_batch_language(
            extraction.provider.as_str(),
            "textDocument/references",
            &extraction.document_symbols,
        )?;
        let workspace_id = self
            .existing_workspace_id(
                store,
                workspace_root_uri,
                language,
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
                file_scope_key,
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

    pub async fn persist_reference_origin_file_batches_with_route_write_batch(
        &self,
        store: &WriteHandle,
        workspace_root_uri: &str,
        extraction: &ReferenceBatchExtraction,
        origin_file_uris: &[String],
    ) -> ExtractResult<PersistenceSummary> {
        let language = document_symbol_batch_language(
            extraction.provider.as_str(),
            "textDocument/references",
            &extraction.document_symbols,
        )?;
        let workspace_id = self
            .existing_workspace_id(
                store,
                workspace_root_uri,
                language,
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
            .persist_reference_origin_file_batches_with_route_write_batch_after_run_started(
                store,
                workspace_id,
                run_id,
                extraction,
                origin_file_uris,
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
        let language = document_symbol_batch_language(
            extraction.provider.as_str(),
            "textDocument/references",
            &extraction.document_symbols,
        )?;
        let route_name = RouteName::references_for_language(language);
        let file_ids = self
            .existing_document_symbol_file_ids(
                store,
                workspace_id,
                language,
                extraction.provider.as_str(),
                "textDocument/references",
                &extraction.document_symbols,
            )
            .await?;
        self.validate_reference_nodes(store, workspace_id, language, extraction)
            .await?;

        store
            .start_route_status(RouteStatusStartInput {
                workspace_id,
                route: route_name.as_str(),
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
                PersistenceRun {
                    workspace_id,
                    run_id,
                },
                ScopedRoute::workspace(workspace_root_uri),
                extraction,
                &file_ids,
                language,
            )
            .await;

        match result {
            Ok(mut summary) => {
                store
                    .complete_route_status(RouteStatusCompleteInput {
                        workspace_id,
                        route: route_name.as_str(),
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
                        route: route_name.as_str(),
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
                        route: route_name.as_str(),
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

    async fn persist_reference_batch_with_route_write_batch_after_run_started(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        run_id: i64,
        workspace_root_uri: &str,
        extraction: &ReferenceBatchExtraction,
    ) -> ExtractResult<PersistenceSummary> {
        let language = document_symbol_batch_language(
            extraction.provider.as_str(),
            "textDocument/references",
            &extraction.document_symbols,
        )?;
        let route_name = RouteName::references_for_language(language);
        let file_ids = self
            .existing_document_symbol_file_ids(
                store,
                workspace_id,
                language,
                extraction.provider.as_str(),
                "textDocument/references",
                &extraction.document_symbols,
            )
            .await?;
        self.validate_reference_nodes(store, workspace_id, language, extraction)
            .await?;

        store
            .start_route_status(RouteStatusStartInput {
                workspace_id,
                route: route_name.as_str(),
                scope: RouteScope::WORKSPACE.as_str(),
                scope_key: workspace_root_uri,
                file_id: None,
                provider: extraction.provider.as_str(),
                provider_version: extraction.provider_version.as_deref(),
                content_hash: Some(&extraction.workspace_fingerprint),
                run_id,
                diagnostics_json: json!({
                    "write_mode": "route_write_batch",
                }),
            })
            .await
            .map_err(ExtractError::storage)?;

        let result = self
            .persist_references_with_route_write_batch_after_scoped_route_started(
                store,
                PersistenceRun {
                    workspace_id,
                    run_id,
                },
                ScopedRoute::workspace(workspace_root_uri),
                extraction,
                &file_ids,
                language,
            )
            .await;

        match result {
            Ok(mut summary) => {
                store
                    .complete_route_status(RouteStatusCompleteInput {
                        workspace_id,
                        route: route_name.as_str(),
                        scope: RouteScope::WORKSPACE.as_str(),
                        scope_key: workspace_root_uri,
                        provider: extraction.provider.as_str(),
                        provider_version: extraction.provider_version.as_deref(),
                        content_hash: Some(&extraction.workspace_fingerprint),
                        run_id,
                        diagnostics_json: json!({
                            "write_mode": "route_write_batch",
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
                        route: route_name.as_str(),
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
                        route: route_name.as_str(),
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

    async fn persist_reference_origin_file_batches_with_route_write_batch_after_run_started(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        run_id: i64,
        extraction: &ReferenceBatchExtraction,
        origin_file_uris: &[String],
    ) -> ExtractResult<PersistenceSummary> {
        let language = document_symbol_batch_language(
            extraction.provider.as_str(),
            "textDocument/references",
            &extraction.document_symbols,
        )?;
        let route_name = RouteName::references_for_language(language);
        let file_ids = self
            .existing_document_symbol_file_ids(
                store,
                workspace_id,
                language,
                extraction.provider.as_str(),
                "textDocument/references",
                &extraction.document_symbols,
            )
            .await?;
        self.validate_reference_nodes(store, workspace_id, language, extraction)
            .await?;

        let mut summary = empty_summary(workspace_id, run_id);
        for origin_file_uri in origin_file_uris {
            let origin_file_id = self
                .existing_file_id_for_uri(
                    store,
                    workspace_id,
                    language,
                    extraction.provider.as_str(),
                    "textDocument/references",
                    origin_file_uri,
                )
                .await?;
            let file_content_hash = file_content_hash_for_scope_key(
                extraction.provider.as_str(),
                "textDocument/references",
                &extraction.document_symbols,
                origin_file_uri,
            )?;
            let filtered_extraction =
                reference_extraction_for_origin_file(extraction, origin_file_uri);

            store
                .start_route_status(RouteStatusStartInput {
                    workspace_id,
                    route: route_name.as_str(),
                    scope: RouteScope::FILE.as_str(),
                    scope_key: origin_file_uri,
                    file_id: Some(origin_file_id),
                    provider: extraction.provider.as_str(),
                    provider_version: extraction.provider_version.as_deref(),
                    content_hash: file_content_hash.as_deref(),
                    run_id,
                    diagnostics_json: json!({
                        "write_mode": "route_write_batch",
                        "incremental_scope": "origin_file",
                    }),
                })
                .await
                .map_err(ExtractError::storage)?;

            let result = self
                .persist_references_with_route_write_batch_after_scoped_route_started(
                    store,
                    PersistenceRun {
                        workspace_id,
                        run_id,
                    },
                    ScopedRoute::file(origin_file_uri),
                    &filtered_extraction,
                    &file_ids,
                    language,
                )
                .await;

            match result {
                Ok(file_summary) => {
                    store
                        .complete_route_status(RouteStatusCompleteInput {
                            workspace_id,
                            route: route_name.as_str(),
                            scope: RouteScope::FILE.as_str(),
                            scope_key: origin_file_uri,
                            provider: extraction.provider.as_str(),
                            provider_version: extraction.provider_version.as_deref(),
                            content_hash: file_content_hash.as_deref(),
                            run_id,
                            diagnostics_json: json!({
                                "write_mode": "route_write_batch",
                                "incremental_scope": "origin_file",
                                "targets_queried": filtered_extraction.summary.targets_queried,
                                "reference_edges": file_summary.reference_edges,
                                "reference_occurrences": file_summary.reference_occurrences,
                                "file_fallbacks": filtered_extraction.summary.file_fallbacks,
                                "skipped_external": filtered_extraction.summary.skipped_external,
                            }),
                        })
                        .await
                        .map_err(ExtractError::storage)?;
                    let stale_edges_closed = store
                        .close_stale_edges_for_route_source_file(CloseStaleRouteInput {
                            workspace_id,
                            run_id,
                            route: route_name.as_str(),
                            scope: RouteScope::FILE.as_str(),
                            scope_key: origin_file_uri,
                            provider: extraction.provider.as_str(),
                        })
                        .await
                        .map_err(ExtractError::storage)?;

                    merge_summary(&mut summary, &file_summary);
                    summary.routes_complete += 1;
                    summary.stale_edges_closed += stale_edges_closed as usize;
                }
                Err(error) => {
                    store
                        .fail_route_status(RouteStatusFailInput {
                            workspace_id,
                            route: route_name.as_str(),
                            scope: RouteScope::FILE.as_str(),
                            scope_key: origin_file_uri,
                            provider: extraction.provider.as_str(),
                            run_id,
                            diagnostics_json: json!({
                                "kind": error.message(),
                                "error": error.to_string(),
                            }),
                        })
                        .await
                        .map_err(ExtractError::storage)?;
                    return Err(error);
                }
            }
        }

        Ok(summary)
    }

    async fn persist_reference_batch_with_document_symbols_after_run_started(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        run_id: i64,
        workspace_root_uri: &str,
        extraction: &ReferenceBatchExtraction,
    ) -> ExtractResult<PersistenceSummary> {
        let language = document_symbol_batch_language(
            extraction.provider.as_str(),
            "textDocument/references",
            &extraction.document_symbols,
        )?;
        let route_name = RouteName::references_for_language(language);
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
                route: route_name.as_str(),
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
                PersistenceRun {
                    workspace_id,
                    run_id,
                },
                ScopedRoute::workspace(workspace_root_uri),
                extraction,
                &file_ids,
                language,
            )
            .await;

        match result {
            Ok(reference_summary) => {
                store
                    .complete_route_status(RouteStatusCompleteInput {
                        workspace_id,
                        route: route_name.as_str(),
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
                        route: route_name.as_str(),
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
                        route: route_name.as_str(),
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
        let language = document_symbol_batch_language(
            extraction.provider.as_str(),
            "textDocument/references",
            &extraction.document_symbols,
        )?;
        let route_name = RouteName::references_for_language(language);
        let file_id = self
            .existing_file_id_for_uri(
                store,
                workspace_id,
                language,
                extraction.provider.as_str(),
                "textDocument/references",
                file_scope_key,
            )
            .await?;
        let (extraction, skipped_missing_source_nodes, skipped_missing_target_nodes) = self
            .reference_file_extraction_with_existing_nodes(
                store,
                workspace_id,
                language,
                file_scope_key,
                extraction,
            )
            .await?;
        let (extraction, file_ids, skipped_missing_occurrence_files) = self
            .reference_file_extraction_with_existing_files(
                store,
                workspace_id,
                language,
                file_scope_key,
                &extraction,
            )
            .await?;
        let extraction = &extraction;
        let file_content_hash = file_content_hash_for_scope_key(
            extraction.provider.as_str(),
            "textDocument/references",
            &extraction.document_symbols,
            file_scope_key,
        )?;

        store
            .start_route_status(RouteStatusStartInput {
                workspace_id,
                route: route_name.as_str(),
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
                PersistenceRun {
                    workspace_id,
                    run_id,
                },
                ScopedRoute::file(file_scope_key),
                extraction,
                &file_ids,
                language,
            )
            .await;

        match result {
            Ok(mut summary) => {
                store
                    .complete_route_status(RouteStatusCompleteInput {
                        workspace_id,
                        route: route_name.as_str(),
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
                            "skipped_missing_source_nodes": skipped_missing_source_nodes,
                            "skipped_missing_target_nodes": skipped_missing_target_nodes,
                            "skipped_missing_occurrence_files": skipped_missing_occurrence_files,
                        }),
                    })
                    .await
                    .map_err(ExtractError::storage)?;
                let stale_edges_closed = store
                    .close_stale_edges_for_route(CloseStaleRouteInput {
                        workspace_id,
                        run_id,
                        route: route_name.as_str(),
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
                        route: route_name.as_str(),
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
        run: PersistenceRun,
        route: ScopedRoute<'_>,
        extraction: &ReferenceBatchExtraction,
        file_ids: &HashMap<String, i64>,
        language: GraphLanguage,
    ) -> ExtractResult<PersistenceSummary> {
        let mut summary = PersistenceSummary {
            workspace_id: run.workspace_id,
            run_id: run.run_id,
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
                    store, run, reference, file_ids, route, language,
                )
                .await?;

            merge_summary(&mut summary, &reference_summary);
        }

        Ok(summary)
    }

    async fn persist_references_with_route_write_batch_after_scoped_route_started(
        &self,
        store: &WriteHandle,
        run: PersistenceRun,
        route: ScopedRoute<'_>,
        extraction: &ReferenceBatchExtraction,
        file_ids: &HashMap<String, i64>,
        language: GraphLanguage,
    ) -> ExtractResult<PersistenceSummary> {
        let provider = extraction.provider.as_str();
        let route_name = RouteName::references_for_language(language);
        let mut batch = RouteWriteBatchInput::default();
        let mut summary = empty_summary(run.workspace_id, run.run_id);

        for reference in &extraction.references {
            let source_node_id = node_id(
                run.workspace_id,
                language.as_store_str(),
                &reference.source_symbol_key,
            );
            let target_node_id = node_id(
                run.workspace_id,
                language.as_store_str(),
                &reference.target_symbol_key,
            );
            let edge_id = edge_id(
                run.workspace_id,
                &source_node_id,
                &target_node_id,
                "references",
                Some("symbol"),
            );
            let lsp_method = reference_lsp_method(reference);

            batch.edges.push(RouteWriteBatchEdgeInput {
                workspace_id: run.workspace_id,
                src_node_id: source_node_id,
                dst_node_id: target_node_id.clone(),
                relation: "references".to_string(),
                context: Some("symbol".to_string()),
                confidence: reference.confidence.clone(),
                confidence_score: reference.confidence_score,
                weight: reference.occurrences.len() as f64,
                properties_json: json!({
                    "provider": reference.provider.as_str(),
                    "route": route_name.as_str(),
                    "source_resolution": reference.source_resolution,
                    "source_symbol_key": reference.source_symbol_key,
                    "target_symbol_key": reference.target_symbol_key,
                }),
                run_id: Some(run.run_id),
            });
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
                let enclosing_node_id =
                    occurrence.enclosing_symbol_key.as_ref().map(|symbol_key| {
                        node_id(run.workspace_id, language.as_store_str(), symbol_key)
                    });

                batch.occurrences.push(RouteWriteBatchOccurrenceInput {
                    node_id: target_node_id.clone(),
                    run_id: run.run_id,
                    file_id,
                    role: "reference".to_string(),
                    range: occurrence.range,
                    enclosing_node_id,
                    raw_json: Some(occurrence.raw_json.clone()),
                });
                batch.edge_evidence.push(RouteWriteBatchEdgeEvidenceInput {
                    edge_id: edge_id.clone(),
                    run_id: run.run_id,
                    provider: reference.provider.as_str().to_string(),
                    lsp_method: Some(lsp_method.clone()),
                    file_id: Some(file_id),
                    range: Some(occurrence.range),
                    raw_json: Some(json!({
                        "edge": reference.raw_json.clone(),
                        "occurrence": occurrence.raw_json.clone(),
                    })),
                });
                batch
                    .route_observations
                    .push(RouteWriteBatchObservationInput {
                        workspace_id: run.workspace_id,
                        run_id: run.run_id,
                        route: route_name.as_str().to_string(),
                        scope: route.scope.to_string(),
                        scope_key: route.scope_key.to_string(),
                        provider: provider.to_string(),
                        entity_kind: "edge".to_string(),
                        entity_id: edge_id.clone(),
                        source_file_id: Some(file_id),
                        properties_json: json!({
                            "source": lsp_method,
                            "source_resolution": reference.source_resolution,
                        }),
                    });

                summary.occurrences += 1;
                summary.reference_occurrences += 1;
                summary.evidence += 1;
            }
        }

        store
            .write_route_batch(batch)
            .await
            .map_err(ExtractError::storage)?;

        Ok(summary)
    }

    pub async fn persist_reference_after_route_started(
        &self,
        store: &WriteHandle,
        run: PersistenceRun,
        workspace_root_uri: &str,
        reference: &ExtractedReference,
        file_ids: &HashMap<String, i64>,
        language: GraphLanguage,
    ) -> ExtractResult<PersistenceSummary> {
        self.persist_reference_after_scoped_route_started(
            store,
            run,
            reference,
            file_ids,
            ScopedRoute::workspace(workspace_root_uri),
            language,
        )
        .await
    }

    async fn persist_reference_after_scoped_route_started(
        &self,
        store: &WriteHandle,
        run: PersistenceRun,
        reference: &ExtractedReference,
        file_ids: &HashMap<String, i64>,
        route: ScopedRoute<'_>,
        language: GraphLanguage,
    ) -> ExtractResult<PersistenceSummary> {
        let provider = reference.provider.as_str();
        let route_name = RouteName::references_for_language(language);
        let lsp_method = reference_lsp_method(reference);
        self.require_node(
            store,
            run.workspace_id,
            language,
            provider,
            "textDocument/references",
            &reference.source_symbol_key,
        )
        .await?;
        self.require_node(
            store,
            run.workspace_id,
            language,
            provider,
            "textDocument/references",
            &reference.target_symbol_key,
        )
        .await?;

        let source_node_id = node_id(
            run.workspace_id,
            language.as_store_str(),
            &reference.source_symbol_key,
        );
        let target_node_id = node_id(
            run.workspace_id,
            language.as_store_str(),
            &reference.target_symbol_key,
        );
        let edge_id = store
            .upsert_edge(EdgeInput {
                workspace_id: run.workspace_id,
                src_node_id: &source_node_id,
                dst_node_id: &target_node_id,
                relation: "references",
                context: Some("symbol"),
                confidence: &reference.confidence,
                confidence_score: reference.confidence_score,
                weight: reference.occurrences.len() as f64,
                properties_json: json!({
                    "provider": reference.provider.as_str(),
                    "route": route_name.as_str(),
                    "source_resolution": reference.source_resolution,
                    "source_symbol_key": reference.source_symbol_key,
                    "target_symbol_key": reference.target_symbol_key,
                }),
                run_id: Some(run.run_id),
            })
            .await
            .map_err(ExtractError::storage)?;

        let mut summary = empty_summary(run.workspace_id, run.run_id);
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
                .map(|symbol_key| node_id(run.workspace_id, language.as_store_str(), symbol_key));

            store
                .insert_occurrence(OccurrenceInput {
                    node_id: &target_node_id,
                    run_id: run.run_id,
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
                    run_id: run.run_id,
                    provider: reference.provider.as_str(),
                    lsp_method: Some(&lsp_method),
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
                    workspace_id: run.workspace_id,
                    run_id: run.run_id,
                    route: route_name.as_str(),
                    scope: route.scope,
                    scope_key: route.scope_key,
                    provider,
                    entity_kind: "edge",
                    entity_id: &edge_id,
                    source_file_id: Some(file_id),
                    properties_json: json!({
                        "source": lsp_method,
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
        if extraction.document_symbols.extractions.is_empty() {
            return Err(ExtractError::response_shape(
                extraction.provider.as_str(),
                "callHierarchy/outgoingCalls",
                "call batch contained no document-symbol files",
            ));
        }
        let workspace_id = store
            .create_workspace(
                workspace_root_uri,
                extraction.document_symbols.language.workspace_kind(),
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
        let language = document_symbol_batch_language(
            extraction.provider.as_str(),
            "callHierarchy/outgoingCalls",
            &extraction.document_symbols,
        )?;
        let workspace_id = self
            .existing_workspace_id(
                store,
                workspace_root_uri,
                language,
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

    pub async fn persist_call_batch_with_route_write_batch(
        &self,
        store: &WriteHandle,
        workspace_root_uri: &str,
        extraction: &CallBatchExtraction,
    ) -> ExtractResult<PersistenceSummary> {
        let language = document_symbol_batch_language(
            extraction.provider.as_str(),
            "callHierarchy/outgoingCalls",
            &extraction.document_symbols,
        )?;
        let workspace_id = self
            .existing_workspace_id(
                store,
                workspace_root_uri,
                language,
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
            .persist_call_batch_with_route_write_batch_after_run_started(
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
        self.persist_call_file_batch_for_file(
            store,
            workspace_root_uri,
            &file_scope_key,
            extraction,
        )
        .await
    }

    pub async fn persist_call_file_batch_for_file(
        &self,
        store: &WriteHandle,
        workspace_root_uri: &str,
        file_scope_key: &str,
        extraction: &CallBatchExtraction,
    ) -> ExtractResult<PersistenceSummary> {
        let language = document_symbol_batch_language(
            extraction.provider.as_str(),
            "callHierarchy/outgoingCalls",
            &extraction.document_symbols,
        )?;
        let workspace_id = self
            .existing_workspace_id(
                store,
                workspace_root_uri,
                language,
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
                file_scope_key,
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

    pub async fn persist_call_origin_file_batches_with_route_write_batch(
        &self,
        store: &WriteHandle,
        workspace_root_uri: &str,
        extraction: &CallBatchExtraction,
        origin_file_uris: &[String],
    ) -> ExtractResult<PersistenceSummary> {
        let language = document_symbol_batch_language(
            extraction.provider.as_str(),
            "callHierarchy/outgoingCalls",
            &extraction.document_symbols,
        )?;
        let workspace_id = self
            .existing_workspace_id(
                store,
                workspace_root_uri,
                language,
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
            .persist_call_origin_file_batches_with_route_write_batch_after_run_started(
                store,
                workspace_id,
                run_id,
                extraction,
                origin_file_uris,
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
        let language = document_symbol_batch_language(
            extraction.provider.as_str(),
            "callHierarchy/outgoingCalls",
            &extraction.document_symbols,
        )?;
        let route_name = RouteName::calls_for_language(language);
        let file_ids = self
            .existing_document_symbol_file_ids(
                store,
                workspace_id,
                language,
                extraction.provider.as_str(),
                "callHierarchy/outgoingCalls",
                &extraction.document_symbols,
            )
            .await?;
        self.validate_call_nodes(store, workspace_id, language, extraction)
            .await?;

        store
            .start_route_status(RouteStatusStartInput {
                workspace_id,
                route: route_name.as_str(),
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
                PersistenceRun {
                    workspace_id,
                    run_id,
                },
                ScopedRoute::workspace(workspace_root_uri),
                extraction,
                &file_ids,
                language,
            )
            .await;

        match result {
            Ok(mut summary) => {
                store
                    .complete_route_status(RouteStatusCompleteInput {
                        workspace_id,
                        route: route_name.as_str(),
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
                        route: route_name.as_str(),
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
                        route: route_name.as_str(),
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

    async fn persist_call_batch_with_route_write_batch_after_run_started(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        run_id: i64,
        workspace_root_uri: &str,
        extraction: &CallBatchExtraction,
    ) -> ExtractResult<PersistenceSummary> {
        let language = document_symbol_batch_language(
            extraction.provider.as_str(),
            "callHierarchy/outgoingCalls",
            &extraction.document_symbols,
        )?;
        let route_name = RouteName::calls_for_language(language);
        let file_ids = self
            .existing_document_symbol_file_ids(
                store,
                workspace_id,
                language,
                extraction.provider.as_str(),
                "callHierarchy/outgoingCalls",
                &extraction.document_symbols,
            )
            .await?;
        self.validate_call_nodes(store, workspace_id, language, extraction)
            .await?;

        store
            .start_route_status(RouteStatusStartInput {
                workspace_id,
                route: route_name.as_str(),
                scope: RouteScope::WORKSPACE.as_str(),
                scope_key: workspace_root_uri,
                file_id: None,
                provider: extraction.provider.as_str(),
                provider_version: extraction.provider_version.as_deref(),
                content_hash: Some(&extraction.workspace_fingerprint),
                run_id,
                diagnostics_json: json!({
                    "write_mode": "route_write_batch",
                }),
            })
            .await
            .map_err(ExtractError::storage)?;

        let result = self
            .persist_calls_with_route_write_batch_after_scoped_route_started(
                store,
                PersistenceRun {
                    workspace_id,
                    run_id,
                },
                ScopedRoute::workspace(workspace_root_uri),
                extraction,
                &file_ids,
                language,
            )
            .await;

        match result {
            Ok(mut summary) => {
                store
                    .complete_route_status(RouteStatusCompleteInput {
                        workspace_id,
                        route: route_name.as_str(),
                        scope: RouteScope::WORKSPACE.as_str(),
                        scope_key: workspace_root_uri,
                        provider: extraction.provider.as_str(),
                        provider_version: extraction.provider_version.as_deref(),
                        content_hash: Some(&extraction.workspace_fingerprint),
                        run_id,
                        diagnostics_json: json!({
                            "write_mode": "route_write_batch",
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
                        route: route_name.as_str(),
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
                        route: route_name.as_str(),
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

    async fn persist_call_origin_file_batches_with_route_write_batch_after_run_started(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        run_id: i64,
        extraction: &CallBatchExtraction,
        origin_file_uris: &[String],
    ) -> ExtractResult<PersistenceSummary> {
        let language = document_symbol_batch_language(
            extraction.provider.as_str(),
            "callHierarchy/outgoingCalls",
            &extraction.document_symbols,
        )?;
        let route_name = RouteName::calls_for_language(language);
        let file_ids = self
            .existing_document_symbol_file_ids(
                store,
                workspace_id,
                language,
                extraction.provider.as_str(),
                "callHierarchy/outgoingCalls",
                &extraction.document_symbols,
            )
            .await?;
        self.validate_call_nodes(store, workspace_id, language, extraction)
            .await?;

        let mut summary = empty_summary(workspace_id, run_id);
        for origin_file_uri in origin_file_uris {
            let origin_file_id = self
                .existing_file_id_for_uri(
                    store,
                    workspace_id,
                    language,
                    extraction.provider.as_str(),
                    "callHierarchy/outgoingCalls",
                    origin_file_uri,
                )
                .await?;
            let file_content_hash = file_content_hash_for_scope_key(
                extraction.provider.as_str(),
                "callHierarchy/outgoingCalls",
                &extraction.document_symbols,
                origin_file_uri,
            )?;
            let filtered_extraction = call_extraction_for_origin_file(extraction, origin_file_uri);

            store
                .start_route_status(RouteStatusStartInput {
                    workspace_id,
                    route: route_name.as_str(),
                    scope: RouteScope::FILE.as_str(),
                    scope_key: origin_file_uri,
                    file_id: Some(origin_file_id),
                    provider: extraction.provider.as_str(),
                    provider_version: extraction.provider_version.as_deref(),
                    content_hash: file_content_hash.as_deref(),
                    run_id,
                    diagnostics_json: json!({
                        "write_mode": "route_write_batch",
                        "incremental_scope": "origin_file",
                    }),
                })
                .await
                .map_err(ExtractError::storage)?;

            let result = self
                .persist_calls_with_route_write_batch_after_scoped_route_started(
                    store,
                    PersistenceRun {
                        workspace_id,
                        run_id,
                    },
                    ScopedRoute::file(origin_file_uri),
                    &filtered_extraction,
                    &file_ids,
                    language,
                )
                .await;

            match result {
                Ok(file_summary) => {
                    store
                        .complete_route_status(RouteStatusCompleteInput {
                            workspace_id,
                            route: route_name.as_str(),
                            scope: RouteScope::FILE.as_str(),
                            scope_key: origin_file_uri,
                            provider: extraction.provider.as_str(),
                            provider_version: extraction.provider_version.as_deref(),
                            content_hash: file_content_hash.as_deref(),
                            run_id,
                            diagnostics_json: json!({
                                "write_mode": "route_write_batch",
                                "incremental_scope": "origin_file",
                                "callable_nodes": filtered_extraction.summary.callable_nodes,
                                "call_edges": file_summary.call_edges,
                                "call_occurrences": file_summary.call_occurrences,
                                "skipped_external_targets": filtered_extraction.summary.skipped_external_targets,
                                "skipped_unresolved_targets": filtered_extraction.summary.skipped_unresolved_targets,
                                "skipped_non_callable_prepare_items": filtered_extraction.summary.skipped_non_callable_prepare_items,
                            }),
                        })
                        .await
                        .map_err(ExtractError::storage)?;
                    let stale_edges_closed = store
                        .close_stale_edges_for_route_source_file(CloseStaleRouteInput {
                            workspace_id,
                            run_id,
                            route: route_name.as_str(),
                            scope: RouteScope::FILE.as_str(),
                            scope_key: origin_file_uri,
                            provider: extraction.provider.as_str(),
                        })
                        .await
                        .map_err(ExtractError::storage)?;

                    merge_summary(&mut summary, &file_summary);
                    summary.routes_complete += 1;
                    summary.stale_edges_closed += stale_edges_closed as usize;
                }
                Err(error) => {
                    store
                        .fail_route_status(RouteStatusFailInput {
                            workspace_id,
                            route: route_name.as_str(),
                            scope: RouteScope::FILE.as_str(),
                            scope_key: origin_file_uri,
                            provider: extraction.provider.as_str(),
                            run_id,
                            diagnostics_json: json!({
                                "kind": error.message(),
                                "error": error.to_string(),
                            }),
                        })
                        .await
                        .map_err(ExtractError::storage)?;
                    return Err(error);
                }
            }
        }

        Ok(summary)
    }

    async fn persist_call_batch_with_document_symbols_after_run_started(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        run_id: i64,
        workspace_root_uri: &str,
        extraction: &CallBatchExtraction,
    ) -> ExtractResult<PersistenceSummary> {
        let language = document_symbol_batch_language(
            extraction.provider.as_str(),
            "callHierarchy/outgoingCalls",
            &extraction.document_symbols,
        )?;
        let route_name = RouteName::calls_for_language(language);
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
                route: route_name.as_str(),
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
                PersistenceRun {
                    workspace_id,
                    run_id,
                },
                ScopedRoute::workspace(workspace_root_uri),
                extraction,
                &file_ids,
                language,
            )
            .await;

        match result {
            Ok(call_summary) => {
                store
                    .complete_route_status(RouteStatusCompleteInput {
                        workspace_id,
                        route: route_name.as_str(),
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
                        route: route_name.as_str(),
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
                        route: route_name.as_str(),
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
        let language = document_symbol_batch_language(
            extraction.provider.as_str(),
            "callHierarchy/outgoingCalls",
            &extraction.document_symbols,
        )?;
        let route_name = RouteName::calls_for_language(language);
        let file_id = self
            .existing_file_id_for_uri(
                store,
                workspace_id,
                language,
                extraction.provider.as_str(),
                "callHierarchy/outgoingCalls",
                file_scope_key,
            )
            .await?;
        let (extraction, skipped_missing_caller_nodes, skipped_missing_callee_nodes) = self
            .call_file_extraction_with_existing_nodes(
                store,
                workspace_id,
                language,
                file_scope_key,
                extraction,
            )
            .await?;
        let (extraction, file_ids, skipped_missing_occurrence_files) = self
            .call_file_extraction_with_existing_files(
                store,
                workspace_id,
                language,
                file_scope_key,
                &extraction,
            )
            .await?;
        let extraction = &extraction;
        let file_content_hash = file_content_hash_for_scope_key(
            extraction.provider.as_str(),
            "callHierarchy/outgoingCalls",
            &extraction.document_symbols,
            file_scope_key,
        )?;

        store
            .start_route_status(RouteStatusStartInput {
                workspace_id,
                route: route_name.as_str(),
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
                PersistenceRun {
                    workspace_id,
                    run_id,
                },
                ScopedRoute::file(file_scope_key),
                extraction,
                &file_ids,
                language,
            )
            .await;

        match result {
            Ok(mut summary) => {
                store
                    .complete_route_status(RouteStatusCompleteInput {
                        workspace_id,
                        route: route_name.as_str(),
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
                            "skipped_missing_caller_nodes": skipped_missing_caller_nodes,
                            "skipped_missing_callee_nodes": skipped_missing_callee_nodes,
                            "skipped_missing_occurrence_files": skipped_missing_occurrence_files,
                        }),
                    })
                    .await
                    .map_err(ExtractError::storage)?;
                let stale_edges_closed = store
                    .close_stale_edges_for_route(CloseStaleRouteInput {
                        workspace_id,
                        run_id,
                        route: route_name.as_str(),
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
                        route: route_name.as_str(),
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
        run: PersistenceRun,
        route: ScopedRoute<'_>,
        extraction: &CallBatchExtraction,
        file_ids: &HashMap<String, i64>,
        language: GraphLanguage,
    ) -> ExtractResult<PersistenceSummary> {
        let mut summary = PersistenceSummary {
            workspace_id: run.workspace_id,
            run_id: run.run_id,
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
                    store, run, call, file_ids, route, language,
                )
                .await?;

            merge_summary(&mut summary, &call_summary);
        }

        Ok(summary)
    }

    async fn persist_calls_with_route_write_batch_after_scoped_route_started(
        &self,
        store: &WriteHandle,
        run: PersistenceRun,
        route: ScopedRoute<'_>,
        extraction: &CallBatchExtraction,
        file_ids: &HashMap<String, i64>,
        language: GraphLanguage,
    ) -> ExtractResult<PersistenceSummary> {
        let provider = extraction.provider.as_str();
        let route_name = RouteName::calls_for_language(language);
        let mut batch = RouteWriteBatchInput::default();
        let mut summary = empty_summary(run.workspace_id, run.run_id);

        for call in &extraction.calls {
            let caller_node_id = node_id(
                run.workspace_id,
                language.as_store_str(),
                &call.caller_symbol_key,
            );
            let callee_node_id = node_id(
                run.workspace_id,
                language.as_store_str(),
                &call.callee_symbol_key,
            );
            let edge_id = edge_id(
                run.workspace_id,
                &caller_node_id,
                &callee_node_id,
                "calls",
                Some(&call.context),
            );
            let lsp_method = call_lsp_method(call);

            batch.edges.push(RouteWriteBatchEdgeInput {
                workspace_id: run.workspace_id,
                src_node_id: caller_node_id,
                dst_node_id: callee_node_id.clone(),
                relation: "calls".to_string(),
                context: Some(call.context.clone()),
                confidence: call.confidence.clone(),
                confidence_score: call.confidence_score,
                weight: call.occurrences.len() as f64,
                properties_json: json!({
                    "provider": call.provider.as_str(),
                    "route": route_name.as_str(),
                    "caller_symbol_key": call.caller_symbol_key,
                    "callee_symbol_key": call.callee_symbol_key,
                }),
                run_id: Some(run.run_id),
            });
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
                let enclosing_node_id = node_id(
                    run.workspace_id,
                    language.as_store_str(),
                    &occurrence.enclosing_symbol_key,
                );

                batch.occurrences.push(RouteWriteBatchOccurrenceInput {
                    node_id: callee_node_id.clone(),
                    run_id: run.run_id,
                    file_id,
                    role: "call".to_string(),
                    range: occurrence.range,
                    enclosing_node_id: Some(enclosing_node_id),
                    raw_json: Some(occurrence.raw_json.clone()),
                });
                batch.edge_evidence.push(RouteWriteBatchEdgeEvidenceInput {
                    edge_id: edge_id.clone(),
                    run_id: run.run_id,
                    provider: call.provider.as_str().to_string(),
                    lsp_method: Some(lsp_method.clone()),
                    file_id: Some(file_id),
                    range: Some(occurrence.range),
                    raw_json: Some(json!({
                        "edge": call.raw_json.clone(),
                        "occurrence": occurrence.raw_json.clone(),
                    })),
                });
                batch
                    .route_observations
                    .push(RouteWriteBatchObservationInput {
                        workspace_id: run.workspace_id,
                        run_id: run.run_id,
                        route: route_name.as_str().to_string(),
                        scope: route.scope.to_string(),
                        scope_key: route.scope_key.to_string(),
                        provider: provider.to_string(),
                        entity_kind: "edge".to_string(),
                        entity_id: edge_id.clone(),
                        source_file_id: Some(file_id),
                        properties_json: json!({
                            "source": lsp_method,
                            "context": call.context,
                        }),
                    });

                summary.occurrences += 1;
                summary.call_occurrences += 1;
                summary.evidence += 1;
            }
        }

        store
            .write_route_batch(batch)
            .await
            .map_err(ExtractError::storage)?;

        Ok(summary)
    }

    pub async fn persist_call_after_route_started(
        &self,
        store: &WriteHandle,
        run: PersistenceRun,
        workspace_root_uri: &str,
        call: &ExtractedCall,
        file_ids: &HashMap<String, i64>,
        language: GraphLanguage,
    ) -> ExtractResult<PersistenceSummary> {
        self.persist_call_after_scoped_route_started(
            store,
            run,
            call,
            file_ids,
            ScopedRoute::workspace(workspace_root_uri),
            language,
        )
        .await
    }

    async fn persist_call_after_scoped_route_started(
        &self,
        store: &WriteHandle,
        run: PersistenceRun,
        call: &ExtractedCall,
        file_ids: &HashMap<String, i64>,
        route: ScopedRoute<'_>,
        language: GraphLanguage,
    ) -> ExtractResult<PersistenceSummary> {
        let provider = call.provider.as_str();
        let route_name = RouteName::calls_for_language(language);
        let lsp_method = call_lsp_method(call);
        self.require_node(
            store,
            run.workspace_id,
            language,
            provider,
            "callHierarchy/outgoingCalls",
            &call.caller_symbol_key,
        )
        .await?;
        self.require_node(
            store,
            run.workspace_id,
            language,
            provider,
            "callHierarchy/outgoingCalls",
            &call.callee_symbol_key,
        )
        .await?;

        let caller_node_id = node_id(
            run.workspace_id,
            language.as_store_str(),
            &call.caller_symbol_key,
        );
        let callee_node_id = node_id(
            run.workspace_id,
            language.as_store_str(),
            &call.callee_symbol_key,
        );
        let edge_id = store
            .upsert_edge(EdgeInput {
                workspace_id: run.workspace_id,
                src_node_id: &caller_node_id,
                dst_node_id: &callee_node_id,
                relation: "calls",
                context: Some(&call.context),
                confidence: &call.confidence,
                confidence_score: call.confidence_score,
                weight: call.occurrences.len() as f64,
                properties_json: json!({
                    "provider": call.provider.as_str(),
                    "route": route_name.as_str(),
                    "caller_symbol_key": call.caller_symbol_key,
                    "callee_symbol_key": call.callee_symbol_key,
                }),
                run_id: Some(run.run_id),
            })
            .await
            .map_err(ExtractError::storage)?;

        let mut summary = empty_summary(run.workspace_id, run.run_id);
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
            let enclosing_node_id = node_id(
                run.workspace_id,
                language.as_store_str(),
                &occurrence.enclosing_symbol_key,
            );

            store
                .insert_occurrence(OccurrenceInput {
                    node_id: &callee_node_id,
                    run_id: run.run_id,
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
                    run_id: run.run_id,
                    provider: call.provider.as_str(),
                    lsp_method: Some(&lsp_method),
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
                    workspace_id: run.workspace_id,
                    run_id: run.run_id,
                    route: route_name.as_str(),
                    scope: route.scope,
                    scope_key: route.scope_key,
                    provider,
                    entity_kind: "edge",
                    entity_id: &edge_id,
                    source_file_id: Some(file_id),
                    properties_json: json!({
                        "source": lsp_method,
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
                        "source_language": file_extraction.source_file.language.as_store_str(),
                        "semantic_language": file_extraction.language.as_store_str(),
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
        language: GraphLanguage,
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
                        "workspace {workspace_root_uri} is missing; run {} first",
                        symbol_prerequisite_commands(language)
                    ),
                )
            })
    }

    async fn existing_document_symbol_file_ids(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        language: GraphLanguage,
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
                            "source file {} is missing from the database; run {} first",
                            file_extraction.source_file.uri,
                            symbol_prerequisite_commands(language)
                        ),
                    )
                })?;
            file_ids.insert(file_extraction.source_file.uri.clone(), file_id);
        }

        Ok(file_ids)
    }

    async fn existing_file_id_for_uri(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        language: GraphLanguage,
        provider: &str,
        method: &str,
        file_uri: &str,
    ) -> ExtractResult<i64> {
        store
            .file_id(workspace_id, file_uri)
            .await
            .map_err(ExtractError::storage)?
            .ok_or_else(|| {
                ExtractError::response_shape(
                    provider,
                    method,
                    format!(
                        "source file {file_uri} is missing from the database; run {} first",
                        symbol_prerequisite_commands(language)
                    ),
                )
            })
    }

    async fn reference_file_extraction_with_existing_nodes(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        language: GraphLanguage,
        file_scope_key: &str,
        extraction: &ReferenceBatchExtraction,
    ) -> ExtractResult<(ReferenceBatchExtraction, usize, usize)> {
        let mut filtered = extraction.clone();
        let mut references = Vec::new();
        let mut skipped_missing_source_nodes = 0;
        let mut skipped_missing_target_nodes = 0;

        for reference in &extraction.references {
            if !self
                .node_exists_for_symbol(store, workspace_id, language, &reference.source_symbol_key)
                .await?
            {
                if symbol_key_belongs_to_file(&reference.source_symbol_key, file_scope_key) {
                    self.require_node(
                        store,
                        workspace_id,
                        language,
                        extraction.provider.as_str(),
                        "textDocument/references",
                        &reference.source_symbol_key,
                    )
                    .await?;
                }

                skipped_missing_source_nodes += 1;
                continue;
            }

            if self
                .node_exists_for_symbol(store, workspace_id, language, &reference.target_symbol_key)
                .await?
            {
                references.push(reference.clone());
            } else if symbol_key_belongs_to_file(&reference.target_symbol_key, file_scope_key) {
                self.require_node(
                    store,
                    workspace_id,
                    language,
                    extraction.provider.as_str(),
                    "textDocument/references",
                    &reference.target_symbol_key,
                )
                .await?;
            } else {
                skipped_missing_target_nodes += 1;
            }
        }

        filtered.references = references;
        Ok((
            filtered,
            skipped_missing_source_nodes,
            skipped_missing_target_nodes,
        ))
    }

    async fn reference_file_extraction_with_existing_files(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        language: GraphLanguage,
        file_scope_key: &str,
        extraction: &ReferenceBatchExtraction,
    ) -> ExtractResult<(ReferenceBatchExtraction, HashMap<String, i64>, usize)> {
        let mut file_ids = HashMap::new();
        let file_id = self
            .existing_file_id_for_uri(
                store,
                workspace_id,
                language,
                extraction.provider.as_str(),
                "textDocument/references",
                file_scope_key,
            )
            .await?;
        file_ids.insert(file_scope_key.to_string(), file_id);

        let mut filtered = extraction.clone();
        let mut references = Vec::new();
        let mut skipped_missing_occurrence_files = 0;

        for reference in &extraction.references {
            let mut reference = reference.clone();
            let mut occurrences = Vec::new();

            for occurrence in &reference.occurrences {
                if file_ids.contains_key(&occurrence.file_uri) {
                    occurrences.push(occurrence.clone());
                    continue;
                }

                match store
                    .file_id(workspace_id, &occurrence.file_uri)
                    .await
                    .map_err(ExtractError::storage)?
                {
                    Some(file_id) => {
                        file_ids.insert(occurrence.file_uri.clone(), file_id);
                        occurrences.push(occurrence.clone());
                    }
                    None => {
                        skipped_missing_occurrence_files += 1;
                    }
                }
            }

            reference.occurrences = occurrences;
            if !reference.occurrences.is_empty() {
                references.push(reference);
            }
        }

        filtered.references = references;
        Ok((filtered, file_ids, skipped_missing_occurrence_files))
    }

    async fn call_file_extraction_with_existing_nodes(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        language: GraphLanguage,
        file_scope_key: &str,
        extraction: &CallBatchExtraction,
    ) -> ExtractResult<(CallBatchExtraction, usize, usize)> {
        let mut filtered = extraction.clone();
        let mut calls = Vec::new();
        let mut skipped_missing_caller_nodes = 0;
        let mut skipped_missing_callee_nodes = 0;

        for call in &extraction.calls {
            if !self
                .node_exists_for_symbol(store, workspace_id, language, &call.caller_symbol_key)
                .await?
            {
                if symbol_key_belongs_to_file(&call.caller_symbol_key, file_scope_key) {
                    self.require_node(
                        store,
                        workspace_id,
                        language,
                        extraction.provider.as_str(),
                        "callHierarchy/outgoingCalls",
                        &call.caller_symbol_key,
                    )
                    .await?;
                }

                skipped_missing_caller_nodes += 1;
                continue;
            }

            if self
                .node_exists_for_symbol(store, workspace_id, language, &call.callee_symbol_key)
                .await?
            {
                calls.push(call.clone());
            } else if symbol_key_belongs_to_file(&call.callee_symbol_key, file_scope_key) {
                self.require_node(
                    store,
                    workspace_id,
                    language,
                    extraction.provider.as_str(),
                    "callHierarchy/outgoingCalls",
                    &call.callee_symbol_key,
                )
                .await?;
            } else {
                skipped_missing_callee_nodes += 1;
            }
        }

        filtered.calls = calls;
        Ok((
            filtered,
            skipped_missing_caller_nodes,
            skipped_missing_callee_nodes,
        ))
    }

    async fn call_file_extraction_with_existing_files(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        language: GraphLanguage,
        file_scope_key: &str,
        extraction: &CallBatchExtraction,
    ) -> ExtractResult<(CallBatchExtraction, HashMap<String, i64>, usize)> {
        let mut file_ids = HashMap::new();
        let file_id = self
            .existing_file_id_for_uri(
                store,
                workspace_id,
                language,
                extraction.provider.as_str(),
                "callHierarchy/outgoingCalls",
                file_scope_key,
            )
            .await?;
        file_ids.insert(file_scope_key.to_string(), file_id);

        let mut filtered = extraction.clone();
        let mut calls = Vec::new();
        let mut skipped_missing_occurrence_files = 0;

        for call in &extraction.calls {
            let mut call = call.clone();
            let mut occurrences = Vec::new();

            for occurrence in &call.occurrences {
                if file_ids.contains_key(&occurrence.file_uri) {
                    occurrences.push(occurrence.clone());
                    continue;
                }

                match store
                    .file_id(workspace_id, &occurrence.file_uri)
                    .await
                    .map_err(ExtractError::storage)?
                {
                    Some(file_id) => {
                        file_ids.insert(occurrence.file_uri.clone(), file_id);
                        occurrences.push(occurrence.clone());
                    }
                    None => {
                        skipped_missing_occurrence_files += 1;
                    }
                }
            }

            call.occurrences = occurrences;
            if !call.occurrences.is_empty() {
                calls.push(call);
            }
        }

        filtered.calls = calls;
        Ok((filtered, file_ids, skipped_missing_occurrence_files))
    }

    async fn validate_reference_nodes(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        language: GraphLanguage,
        extraction: &ReferenceBatchExtraction,
    ) -> ExtractResult<()> {
        for reference in &extraction.references {
            self.require_node(
                store,
                workspace_id,
                language,
                extraction.provider.as_str(),
                "textDocument/references",
                &reference.source_symbol_key,
            )
            .await?;
            self.require_node(
                store,
                workspace_id,
                language,
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
        language: GraphLanguage,
        extraction: &CallBatchExtraction,
    ) -> ExtractResult<()> {
        for call in &extraction.calls {
            self.require_node(
                store,
                workspace_id,
                language,
                extraction.provider.as_str(),
                "callHierarchy/outgoingCalls",
                &call.caller_symbol_key,
            )
            .await?;
            self.require_node(
                store,
                workspace_id,
                language,
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
        language: GraphLanguage,
        provider: &str,
        method: &str,
        symbol_key: &str,
    ) -> ExtractResult<()> {
        let id = node_id(workspace_id, language.as_store_str(), symbol_key);
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
                "symbol node {symbol_key} is missing from the database; run {} first",
                symbol_prerequisite_commands(language)
            ),
        ))
    }

    async fn node_exists_for_symbol(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        language: GraphLanguage,
        symbol_key: &str,
    ) -> ExtractResult<bool> {
        let id = node_id(workspace_id, language.as_store_str(), symbol_key);
        store.node_exists(&id).await.map_err(ExtractError::storage)
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

    async fn persist_batch_with_write_batch_after_run_started(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        run_id: i64,
        extraction: &DocumentSymbolBatchExtraction,
        close_stale: bool,
    ) -> ExtractResult<PersistenceSummary> {
        let mut batch = DocumentSymbolWriteBatchInput::default();
        let mut summary = empty_summary(workspace_id, run_id);

        for file_extraction in &extraction.extractions {
            self.push_document_symbol_file_write_batch(
                &mut batch,
                &mut summary,
                workspace_id,
                run_id,
                file_extraction,
                close_stale,
            )?;
        }

        let batch_summary = store
            .write_document_symbol_batch(batch)
            .await
            .map_err(ExtractError::storage)?;
        summary.stale_nodes_closed = batch_summary.stale_nodes_closed as usize;
        summary.stale_edges_closed = batch_summary.stale_edges_closed as usize;

        Ok(summary)
    }

    fn push_document_symbol_file_write_batch(
        &self,
        batch: &mut DocumentSymbolWriteBatchInput,
        summary: &mut PersistenceSummary,
        workspace_id: i64,
        run_id: i64,
        extraction: &DocumentSymbolExtraction,
        close_stale: bool,
    ) -> ExtractResult<()> {
        let route_name = RouteName::document_symbols_for_language(extraction.language);
        let file_uri = &extraction.source_file.uri;
        let file_node_id = node_id(
            workspace_id,
            extraction.language.as_store_str(),
            &extraction.source_file.file_symbol_key,
        );
        let file_name = basename_from_relative_path(&extraction.source_file.relative_path);
        let mut node_ids = HashMap::new();
        node_ids.insert(
            extraction.source_file.file_symbol_key.clone(),
            file_node_id.clone(),
        );

        batch.files.push(DocumentSymbolWriteBatchFileInput {
            workspace_id,
            uri: file_uri.clone(),
            path: extraction.source_file.relative_path.clone(),
            language: extraction.source_file.language.as_store_str().to_string(),
            content_hash: extraction.source_file.content_hash.clone(),
            last_seen_run_id: Some(run_id),
            properties_json: json!({
                "provider": extraction.provider.as_str(),
                "source_language": extraction.source_file.language.as_store_str(),
                "semantic_language": extraction.language.as_store_str(),
                "raw_metadata": extraction.raw_metadata,
            }),
        });
        batch
            .route_status_starts
            .push(DocumentSymbolWriteBatchRouteStatusStartInput {
                workspace_id,
                route: route_name.as_str().to_string(),
                scope: RouteScope::FILE.as_str().to_string(),
                scope_key: file_uri.clone(),
                file_uri: Some(file_uri.clone()),
                provider: extraction.provider.as_str().to_string(),
                provider_version: extraction.provider_version.clone(),
                content_hash: extraction.source_file.content_hash.clone(),
                run_id,
                diagnostics_json: json!({
                    "write_mode": "document_symbol_write_batch",
                }),
            });

        batch.nodes.push(DocumentSymbolWriteBatchNodeInput {
            workspace_id,
            language: extraction.language.as_store_str().to_string(),
            kind: "file".to_string(),
            name: file_name.clone(),
            qualified_name: Some(extraction.source_file.relative_path.clone()),
            display_name: Some(file_name),
            symbol_key: extraction.source_file.file_symbol_key.clone(),
            file_uri: Some(file_uri.clone()),
            range: None,
            selection_range: None,
            container_node_id: None,
            properties_json: json!({
                "provider": extraction.provider.as_str(),
                "source_language": extraction.source_file.language.as_store_str(),
                "semantic_language": extraction.language.as_store_str(),
                "uri": extraction.source_file.uri,
            }),
            run_id: Some(run_id),
        });
        batch
            .route_observations
            .push(DocumentSymbolWriteBatchObservationInput {
                workspace_id,
                run_id,
                route: route_name.as_str().to_string(),
                scope: RouteScope::FILE.as_str().to_string(),
                scope_key: file_uri.clone(),
                provider: extraction.provider.as_str().to_string(),
                entity_kind: "node".to_string(),
                entity_id: file_node_id.clone(),
                source_file_uri: Some(file_uri.clone()),
                properties_json: json!({
                    "source": "textDocument/documentSymbol",
                }),
            });

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
            let node_id = node_id(
                workspace_id,
                symbol.language.as_store_str(),
                &symbol.symbol_key,
            );

            batch.nodes.push(DocumentSymbolWriteBatchNodeInput {
                workspace_id,
                language: symbol.language.as_store_str().to_string(),
                kind: symbol.kind.clone(),
                name: symbol.name.clone(),
                qualified_name: symbol.qualified_name.clone(),
                display_name: Some(symbol.name.clone()),
                symbol_key: symbol.symbol_key.clone(),
                file_uri: Some(file_uri.clone()),
                range: Some(symbol.range),
                selection_range: Some(symbol.selection_range),
                container_node_id: container_node_id.clone(),
                properties_json: symbol_properties_json(symbol),
                run_id: Some(run_id),
            });
            batch
                .route_observations
                .push(DocumentSymbolWriteBatchObservationInput {
                    workspace_id,
                    run_id,
                    route: route_name.as_str().to_string(),
                    scope: RouteScope::FILE.as_str().to_string(),
                    scope_key: file_uri.clone(),
                    provider: extraction.provider.as_str().to_string(),
                    entity_kind: "node".to_string(),
                    entity_id: node_id.clone(),
                    source_file_uri: Some(file_uri.clone()),
                    properties_json: json!({
                        "source": "textDocument/documentSymbol",
                    }),
                });
            batch
                .occurrences
                .push(DocumentSymbolWriteBatchOccurrenceInput {
                    node_id: node_id.clone(),
                    run_id,
                    file_uri: file_uri.clone(),
                    role: "definition".to_string(),
                    range: symbol.selection_range,
                    enclosing_node_id: container_node_id,
                    raw_json: Some(symbol.raw_json.clone()),
                });

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
            let edge_id = edge_id(
                workspace_id,
                &src_node_id,
                &dst_node_id,
                &relation.relation,
                None,
            );

            batch.edges.push(RouteWriteBatchEdgeInput {
                workspace_id,
                src_node_id,
                dst_node_id,
                relation: relation.relation.clone(),
                context: None,
                confidence: relation.confidence.clone(),
                confidence_score: relation.confidence_score,
                weight: 1.0,
                properties_json: json!({
                    "provider": relation.provider.as_str(),
                    "source": "textDocument/documentSymbol",
                }),
                run_id: Some(run_id),
            });
            batch
                .route_observations
                .push(DocumentSymbolWriteBatchObservationInput {
                    workspace_id,
                    run_id,
                    route: route_name.as_str().to_string(),
                    scope: RouteScope::FILE.as_str().to_string(),
                    scope_key: file_uri.clone(),
                    provider: extraction.provider.as_str().to_string(),
                    entity_kind: "edge".to_string(),
                    entity_id: edge_id.clone(),
                    source_file_uri: Some(file_uri.clone()),
                    properties_json: json!({
                        "relation": "contains",
                        "source": "textDocument/documentSymbol",
                    }),
                });
            batch
                .edge_evidence
                .push(DocumentSymbolWriteBatchEdgeEvidenceInput {
                    edge_id,
                    run_id,
                    provider: relation.provider.as_str().to_string(),
                    lsp_method: Some("textDocument/documentSymbol".to_string()),
                    file_uri: Some(file_uri.clone()),
                    range: relation.range,
                    raw_json: Some(relation.raw_json.clone()),
                });
        }

        batch
            .route_status_completes
            .push(DocumentSymbolWriteBatchRouteStatusCompleteInput {
                workspace_id,
                route: route_name.as_str().to_string(),
                scope: RouteScope::FILE.as_str().to_string(),
                scope_key: file_uri.clone(),
                provider: extraction.provider.as_str().to_string(),
                provider_version: extraction.provider_version.clone(),
                content_hash: extraction.source_file.content_hash.clone(),
                run_id,
                diagnostics_json: json!({
                    "write_mode": "document_symbol_write_batch",
                    "files": 1,
                    "nodes": node_ids.len(),
                    "contains_edges": extraction.relations.len(),
                    "occurrences": extraction.symbols.len(),
                    "evidence": extraction.relations.len(),
                }),
            });
        if close_stale {
            batch
                .close_stale_nodes
                .push(DocumentSymbolWriteBatchCloseStaleRouteInput {
                    workspace_id,
                    run_id,
                    route: route_name.as_str().to_string(),
                    scope: RouteScope::FILE.as_str().to_string(),
                    scope_key: file_uri.clone(),
                    provider: extraction.provider.as_str().to_string(),
                });
            batch
                .close_stale_edges
                .push(DocumentSymbolWriteBatchCloseStaleRouteInput {
                    workspace_id,
                    run_id,
                    route: route_name.as_str().to_string(),
                    scope: RouteScope::FILE.as_str().to_string(),
                    scope_key: file_uri.clone(),
                    provider: extraction.provider.as_str().to_string(),
                });
        }

        summary.files += 1;
        summary.nodes += node_ids.len();
        summary.edges += extraction.relations.len();
        summary.occurrences += extraction.symbols.len();
        summary.evidence += extraction.relations.len();
        summary.routes_complete += 1;

        Ok(())
    }

    async fn persist_after_run_started(
        &self,
        store: &WriteHandle,
        workspace_id: i64,
        run_id: i64,
        extraction: &DocumentSymbolExtraction,
    ) -> ExtractResult<PersistenceSummary> {
        let route_name = RouteName::document_symbols_for_language(extraction.language);
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
                    "source_language": extraction.source_file.language.as_store_str(),
                    "semantic_language": extraction.language.as_store_str(),
                    "raw_metadata": extraction.raw_metadata,
                }),
            })
            .await
            .map_err(ExtractError::storage)?;

        store
            .start_route_status(RouteStatusStartInput {
                workspace_id,
                route: route_name.as_str(),
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
                        route: route_name.as_str(),
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
                        route: route_name.as_str(),
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
                        route: route_name.as_str(),
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
                        route: route_name.as_str(),
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
                language: extraction.language.as_store_str(),
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
                    "source_language": extraction.source_file.language.as_store_str(),
                    "semantic_language": extraction.language.as_store_str(),
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
        let route_name = RouteName::document_symbols_for_language(extraction.language);
        store
            .record_route_observation(RouteObservationInput {
                workspace_id,
                run_id,
                route: route_name.as_str(),
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
        let route_name = RouteName::document_symbols_for_language(extraction.language);
        store
            .record_route_observation(RouteObservationInput {
                workspace_id,
                run_id,
                route: route_name.as_str(),
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

fn file_content_hash_for_scope_key(
    provider: &str,
    method: &str,
    extraction: &DocumentSymbolBatchExtraction,
    file_scope_key: &str,
) -> ExtractResult<Option<String>> {
    extraction
        .extractions
        .iter()
        .find(|file_extraction| file_extraction.source_file.uri == file_scope_key)
        .map(|file_extraction| file_extraction.source_file.content_hash.clone())
        .ok_or_else(|| {
            ExtractError::response_shape(
                provider,
                method,
                format!("source file {file_scope_key} is missing from the relation context"),
            )
        })
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

fn document_symbol_batch_language(
    provider: &str,
    method: &str,
    extraction: &DocumentSymbolBatchExtraction,
) -> ExtractResult<GraphLanguage> {
    if extraction.extractions.is_empty() {
        return Err(ExtractError::response_shape(
            provider,
            method,
            "relation batch contained no document-symbol files",
        ));
    }

    for file_extraction in &extraction.extractions {
        if file_extraction.language != extraction.language {
            return Err(ExtractError::response_shape(
                provider,
                method,
                "relation batch contained mixed document-symbol languages",
            ));
        }
        for symbol in &file_extraction.symbols {
            if symbol.language != extraction.language {
                return Err(ExtractError::response_shape(
                    provider,
                    method,
                    "relation batch contained mixed symbol languages",
                ));
            }
        }
    }

    Ok(extraction.language)
}

fn symbol_prerequisite_commands(language: GraphLanguage) -> &'static str {
    match language {
        GraphLanguage::Rust => {
            "rust-workspace --symbols, rust-crate --symbols, or rust-file --symbols"
        }
        GraphLanguage::CSharp => {
            "csharp-solution --symbols, csharp-project --symbols, or csharp-file --symbols"
        }
        GraphLanguage::Soul => "soul-workspace --symbols or soul-file --symbols",
    }
}

fn reference_extraction_for_origin_file(
    extraction: &ReferenceBatchExtraction,
    origin_file_uri: &str,
) -> ReferenceBatchExtraction {
    let references = extraction
        .references
        .iter()
        .filter_map(|reference| {
            let mut reference = reference.clone();
            reference.occurrences = reference
                .occurrences
                .into_iter()
                .filter(|occurrence| occurrence.file_uri == origin_file_uri)
                .collect::<Vec<_>>();
            if reference.occurrences.is_empty() {
                None
            } else {
                Some(reference)
            }
        })
        .collect::<Vec<_>>();
    let reference_occurrences = references
        .iter()
        .map(|reference| reference.occurrences.len())
        .sum();
    let reference_edges = references.len();
    let file_fallbacks = references
        .iter()
        .filter(|reference| reference.source_resolution == "file_fallback")
        .count();

    ReferenceBatchExtraction {
        provider: extraction.provider,
        provider_version: extraction.provider_version.clone(),
        workspace_fingerprint: extraction.workspace_fingerprint.clone(),
        document_symbols: extraction.document_symbols.clone(),
        references,
        summary: ReferenceRouteSummary {
            targets_queried: extraction.summary.targets_queried,
            reference_edges,
            reference_occurrences,
            file_fallbacks,
            skipped_external: 0,
        },
        raw_metadata: extraction.raw_metadata.clone(),
    }
}

fn call_extraction_for_origin_file(
    extraction: &CallBatchExtraction,
    origin_file_uri: &str,
) -> CallBatchExtraction {
    let calls = extraction
        .calls
        .iter()
        .filter_map(|call| {
            let mut call = call.clone();
            call.occurrences = call
                .occurrences
                .into_iter()
                .filter(|occurrence| occurrence.file_uri == origin_file_uri)
                .collect::<Vec<_>>();
            if call.occurrences.is_empty() {
                None
            } else {
                Some(call)
            }
        })
        .collect::<Vec<_>>();
    let call_occurrences = calls.iter().map(|call| call.occurrences.len()).sum();
    let callable_nodes = calls
        .iter()
        .map(|call| call.caller_symbol_key.clone())
        .collect::<HashSet<_>>()
        .len();
    let call_edges = calls.len();

    CallBatchExtraction {
        provider: extraction.provider,
        provider_version: extraction.provider_version.clone(),
        workspace_fingerprint: extraction.workspace_fingerprint.clone(),
        document_symbols: extraction.document_symbols.clone(),
        calls,
        summary: CallRouteSummary {
            callable_nodes,
            call_edges,
            call_occurrences,
            skipped_external_targets: 0,
            skipped_unresolved_targets: 0,
            skipped_non_callable_prepare_items: 0,
        },
        raw_metadata: extraction.raw_metadata.clone(),
    }
}

fn reference_lsp_method(reference: &ExtractedReference) -> String {
    lsp_method_from_raw(&reference.raw_json, "textDocument/references")
}

fn call_lsp_method(call: &ExtractedCall) -> String {
    lsp_method_from_raw(&call.raw_json, "callHierarchy/outgoingCalls")
}

fn lsp_method_from_raw(raw_json: &Value, default_method: &str) -> String {
    raw_json
        .get("lsp_method")
        .and_then(Value::as_str)
        .unwrap_or(default_method)
        .to_string()
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
