use crate::{
    ExtractError, ExtractResult,
    benchmark::{BenchmarkSummary, Stopwatch},
    document_symbols::paths::file_uri,
    model::{
        CallRouteSummary, DocumentSymbolBatchExtraction, DocumentSymbolBatchRequest,
        ReferenceRouteSummary, RouteName, RouteScope,
    },
    persist::{ExtractionPersister, PersistenceSummary},
    provider::DocumentSymbolProvider,
    providers::rust_analyzer::RustAnalyzerProvider,
    workspace_all::{
        FileRelationContext, FileRelationRouteStart, FileRelationWorkerSummary,
        ThreadedWorkspaceAllConfig, WorkspaceAllSummary,
    },
};

use semantic_graph_db_manager::{
    CloseStaleRouteInput, RouteStatusCompleteInput, RouteStatusFailInput, RouteStatusStartInput,
    WriteHandle,
};

use serde_json::json;
use sha2::Digest;
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::{sync::Mutex, task::JoinError};

pub struct ThreadedWorkspaceAllRunner;

impl ThreadedWorkspaceAllRunner {
    pub async fn run(
        store: &WriteHandle,
        provider: &RustAnalyzerProvider,
        document_request: DocumentSymbolBatchRequest,
        config: ThreadedWorkspaceAllConfig,
    ) -> ExtractResult<WorkspaceAllSummary> {
        let total_timer = Stopwatch::start_new();
        let mut benchmark = BenchmarkSummary::new();
        let analysis_worker_count = effective_analysis_worker_count(&config);
        benchmark.insert_label("threaded.actual_execution_mode", execution_mode_label());
        benchmark.insert_count("threaded.reference_jobs", config.reference_jobs());
        benchmark.insert_count("threaded.call_jobs", config.call_jobs());
        benchmark.insert_count("threaded.analysis_workers", config.analysis_workers());
        benchmark.insert_count(
            "threaded.reference_analysis_workers",
            config.reference_analysis_workers(),
        );
        benchmark.insert_count(
            "threaded.call_analysis_workers",
            config.call_analysis_workers(),
        );
        benchmark.insert_count("threaded.effective_analysis_workers", analysis_worker_count);
        benchmark.insert_count("threaded.input_files", document_request.file_paths.len());

        let workspace_root_uri_timer = Stopwatch::start_new();
        let workspace_root_uri = file_uri(&document_request.workspace_root)?;
        benchmark.insert_duration_ms(
            "threaded.workspace_root_uri",
            workspace_root_uri_timer.elapsed(),
        );

        let analysis_pool_timer = Stopwatch::start_new();
        let analysis_pool = rust_analyzer_lib::AnalysisWorkerPool::start(
            &document_request.workspace_root,
            analysis_worker_count,
        )
        .map_err(|source| ExtractError::rust_analyzer_lib("start analysis pool", source))?;
        benchmark.insert_duration_ms(
            "threaded.analysis_pool_start",
            analysis_pool_timer.elapsed(),
        );
        benchmark.insert_count(
            "threaded.analysis_pool_workers",
            analysis_pool.worker_count(),
        );

        let document_symbols_query_timer = Stopwatch::start_new();
        let document_symbol_items = analysis_pool
            .document_symbols_for_files(document_request.file_paths.clone())
            .await
            .map_err(|source| {
                ExtractError::rust_analyzer_lib(
                    "rust-analyzer-lib document_symbols_for_files",
                    source,
                )
            })?;
        benchmark.insert_duration_ms(
            "threaded.document_symbols_query",
            document_symbols_query_timer.elapsed(),
        );

        let document_symbols_map_timer = Stopwatch::start_new();
        let document_symbols =
            provider.map_document_symbol_items(document_request.clone(), document_symbol_items)?;
        benchmark.insert_count(
            "threaded.document_files",
            document_symbols.extractions.len(),
        );
        benchmark.insert_count("threaded.document_symbols", symbol_count(&document_symbols));
        benchmark.insert_duration_ms(
            "threaded.document_symbols_map",
            document_symbols_map_timer.elapsed(),
        );

        let document_symbols_persist_timer = Stopwatch::start_new();
        let document_summary = ExtractionPersister
            .persist_document_symbol_batch(store, &workspace_root_uri, &document_symbols)
            .await?;
        benchmark.insert_duration_ms(
            "threaded.document_symbols_persist",
            document_symbols_persist_timer.elapsed(),
        );
        let workspace_id = document_summary.workspace_id;

        let file_ids_timer = Stopwatch::start_new();
        let file_ids =
            Arc::new(file_ids_for_document_symbols(store, workspace_id, &document_symbols).await?);
        benchmark.insert_duration_ms("threaded.file_ids_lookup", file_ids_timer.elapsed());

        let workspace_fingerprint_timer = Stopwatch::start_new();
        let workspace_fingerprint = workspace_fingerprint(&document_symbols);
        benchmark.insert_duration_ms(
            "threaded.workspace_fingerprint",
            workspace_fingerprint_timer.elapsed(),
        );

        let targets_timer = Stopwatch::start_new();
        let reference_targets = provider
            .reference_targets_for_document_symbols(&document_request, &document_symbols)?;
        let call_targets =
            provider.call_targets_for_document_symbols(&document_request, &document_symbols)?;
        benchmark.insert_count("threaded.reference_targets", reference_targets.len());
        benchmark.insert_count("threaded.call_targets", call_targets.len());
        benchmark.insert_duration_ms("threaded.targets_build", targets_timer.elapsed());

        let file_work_timer = Stopwatch::start_new();
        let file_work_items = file_semantic_work_items(reference_targets, call_targets);
        benchmark.insert_count("threaded.file_work_items", file_work_items.len());
        benchmark.insert_duration_ms("threaded.file_work_build", file_work_timer.elapsed());

        let reference_run_id = store
            .start_run(
                workspace_id,
                provider.provider_id().as_str(),
                document_symbols.provider_version.as_deref(),
                None,
            )
            .await
            .map_err(ExtractError::storage)?;
        let call_run_id = store
            .start_run(
                workspace_id,
                provider.provider_id().as_str(),
                document_symbols.provider_version.as_deref(),
                None,
            )
            .await
            .map_err(ExtractError::storage)?;
        let route_start = FileRelationRouteStart {
            store,
            workspace_id,
            workspace_root_uri: &workspace_root_uri,
            provider,
            document_symbols: &document_symbols,
            workspace_fingerprint: &workspace_fingerprint,
            analysis_workers: analysis_worker_count,
            file_work_items: file_work_items.len(),
        };
        start_relation_route(
            &route_start,
            RouteName::RUST_REFERENCES.as_str(),
            reference_run_id,
            config.reference_jobs(),
        )
        .await?;
        start_relation_route(
            &route_start,
            RouteName::RUST_CALLS.as_str(),
            call_run_id,
            config.call_jobs(),
        )
        .await?;

        let worker_handles = analysis_pool.worker_handles();
        let analysis_worker = worker_handles.first().cloned().ok_or_else(|| {
            ExtractError::response_shape(
                "rust-analyzer",
                "rust-workspace-all",
                "analysis pool contained no workers",
            )
        })?;
        let relation_context = FileRelationContext {
            store: store.clone(),
            provider: provider.clone(),
            analysis_worker,
            document_request,
            document_symbols,
            file_ids,
            workspace_id,
            workspace_root_uri,
            workspace_fingerprint,
            reference_run_id,
            call_run_id,
            analysis_worker_count,
        };

        let relations_timer = Stopwatch::start_new();
        let relation_result =
            run_file_relation_workers(relation_context.clone(), file_work_items, worker_handles)
                .await;
        benchmark.insert_duration_ms("threaded.file_relations", relations_timer.elapsed());

        let analysis_pool_shutdown_timer = Stopwatch::start_new();
        analysis_pool
            .shutdown()
            .await
            .map_err(|source| ExtractError::rust_analyzer_lib("shutdown analysis pool", source))?;
        benchmark.insert_duration_ms(
            "threaded.analysis_pool_shutdown",
            analysis_pool_shutdown_timer.elapsed(),
        );

        let (mut reference_summary, reference_route_summary, mut call_summary, call_route_summary) =
            match relation_result {
                Ok(mut summary) => {
                    complete_reference_route(
                        &relation_context,
                        &mut summary.reference_persistence,
                        &summary.reference_route,
                    )
                    .await?;
                    complete_call_route(
                        &relation_context,
                        &mut summary.call_persistence,
                        &summary.call_route,
                    )
                    .await?;
                    (
                        summary.reference_persistence,
                        summary.reference_route,
                        summary.call_persistence,
                        summary.call_route,
                    )
                }
                Err(error) => {
                    fail_file_relation_routes(&relation_context, &error).await?;
                    return Err(error);
                }
            };

        reference_summary.routes_complete = 1;
        call_summary.routes_complete = 1;
        benchmark.insert_duration_ms("threaded.total", total_timer.elapsed());

        Ok(WorkspaceAllSummary {
            benchmark,
            document_summary,
            reference_summary,
            call_summary,
            reference_route_summary,
            call_route_summary,
        })
    }
}

async fn start_relation_route(
    route_start: &FileRelationRouteStart<'_>,
    route: &str,
    run_id: i64,
    requested_jobs: usize,
) -> ExtractResult<()> {
    route_start
        .store
        .start_route_status(RouteStatusStartInput {
            workspace_id: route_start.workspace_id,
            route,
            scope: RouteScope::WORKSPACE.as_str(),
            scope_key: route_start.workspace_root_uri,
            file_id: None,
            provider: route_start.provider.provider_id().as_str(),
            provider_version: route_start.document_symbols.provider_version.as_deref(),
            content_hash: Some(route_start.workspace_fingerprint),
            run_id,
            diagnostics_json: execution_diagnostics(
                requested_jobs,
                route_start.analysis_workers,
                route_start.file_work_items,
            ),
        })
        .await
        .map(|_route_status_id| ())
        .map_err(ExtractError::storage)
}

async fn run_file_relation_workers(
    context: FileRelationContext,
    file_work_items: Vec<rust_analyzer_lib::FileSemanticWork>,
    worker_handles: Vec<rust_analyzer_lib::AnalysisWorkerHandle>,
) -> ExtractResult<FileRelationWorkerSummary> {
    let queue = Arc::new(Mutex::new(VecDeque::from(file_work_items)));
    let failed = Arc::new(AtomicBool::new(false));
    let mut handles = Vec::with_capacity(worker_handles.len());
    let workspace_id = context.workspace_id;
    let reference_run_id = context.reference_run_id;
    let call_run_id = context.call_run_id;

    for analysis_worker in worker_handles {
        let mut worker_context = context.clone();
        worker_context.analysis_worker = analysis_worker;
        let worker_queue = Arc::clone(&queue);
        let worker_failed = Arc::clone(&failed);
        let worker_failed_for_result = Arc::clone(&worker_failed);
        handles.push(tokio::spawn(async move {
            let result = file_relation_worker(worker_context, worker_queue, worker_failed).await;
            if result.is_err() {
                worker_failed_for_result.store(true, Ordering::SeqCst);
            }
            result
        }));
    }

    collect_file_relation_workers(handles, workspace_id, reference_run_id, call_run_id).await
}

async fn file_relation_worker(
    context: FileRelationContext,
    queue: Arc<Mutex<VecDeque<rust_analyzer_lib::FileSemanticWork>>>,
    failed: Arc<AtomicBool>,
) -> ExtractResult<FileRelationWorkerSummary> {
    let mut summary = empty_file_relation_worker_summary(
        context.workspace_id,
        context.reference_run_id,
        context.call_run_id,
    );

    loop {
        if failed.load(Ordering::SeqCst) {
            return Ok(summary);
        }

        let work = {
            let mut queue = queue.lock().await;
            queue.pop_front()
        };
        let Some(work) = work else {
            return Ok(summary);
        };

        let file_result = context
            .analysis_worker
            .file_semantic_work(work)
            .await
            .map_err(|source| {
                ExtractError::rust_analyzer_lib("rust-analyzer-lib file_semantic_work", source)
            })?;

        let reference_target_count = file_result.reference_sets.len();
        if reference_target_count > 0 {
            let extraction = context.provider.map_reference_sets(
                &context.document_request,
                context.document_symbols.clone(),
                file_result.reference_sets,
                reference_target_count,
            )?;

            for reference in &extraction.references {
                let reference_summary = ExtractionPersister
                    .persist_reference_after_route_started(
                        &context.store,
                        context.workspace_id,
                        context.reference_run_id,
                        &context.workspace_root_uri,
                        reference,
                        &context.file_ids,
                    )
                    .await?;
                merge_summary(&mut summary.reference_persistence, &reference_summary);
            }

            summary.reference_route.targets_queried += extraction.summary.targets_queried;
            summary.reference_route.reference_edges += extraction.summary.reference_edges;
            summary.reference_route.reference_occurrences +=
                extraction.summary.reference_occurrences;
            summary.reference_route.file_fallbacks += extraction.summary.file_fallbacks;
            summary.reference_route.skipped_external += extraction.summary.skipped_external;
        }

        let call_target_count = file_result.call_sets.len();
        if call_target_count > 0 {
            let extraction = context.provider.map_call_sets(
                &context.document_request,
                context.document_symbols.clone(),
                file_result.call_sets,
                call_target_count,
            )?;

            for call in &extraction.calls {
                let call_summary = ExtractionPersister
                    .persist_call_after_route_started(
                        &context.store,
                        context.workspace_id,
                        context.call_run_id,
                        &context.workspace_root_uri,
                        call,
                        &context.file_ids,
                    )
                    .await?;
                merge_summary(&mut summary.call_persistence, &call_summary);
            }

            summary.call_route.callable_nodes += extraction.summary.callable_nodes;
            summary.call_route.call_edges += extraction.summary.call_edges;
            summary.call_route.call_occurrences += extraction.summary.call_occurrences;
            summary.call_route.skipped_external_targets +=
                extraction.summary.skipped_external_targets;
            summary.call_route.skipped_unresolved_targets +=
                extraction.summary.skipped_unresolved_targets;
            summary.call_route.skipped_non_callable_prepare_items +=
                extraction.summary.skipped_non_callable_prepare_items;
        }
    }
}

async fn collect_file_relation_workers(
    handles: Vec<tokio::task::JoinHandle<ExtractResult<FileRelationWorkerSummary>>>,
    workspace_id: i64,
    reference_run_id: i64,
    call_run_id: i64,
) -> ExtractResult<FileRelationWorkerSummary> {
    let mut summary =
        empty_file_relation_worker_summary(workspace_id, reference_run_id, call_run_id);
    let mut first_error = None;

    for handle in handles {
        match handle.await {
            Ok(Ok(worker_summary)) => {
                merge_file_relation_summary(&mut summary, &worker_summary);
            }
            Ok(Err(error)) => {
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
            Err(error) => {
                if first_error.is_none() {
                    first_error = Some(worker_join_error("rust.file_relations", error));
                }
            }
        }
    }

    match first_error {
        Some(error) => Err(error),
        None => Ok(summary),
    }
}

async fn complete_reference_route(
    context: &FileRelationContext,
    summary: &mut PersistenceSummary,
    route_summary: &ReferenceRouteSummary,
) -> ExtractResult<()> {
    context
        .store
        .complete_route_status(RouteStatusCompleteInput {
            workspace_id: context.workspace_id,
            route: RouteName::RUST_REFERENCES.as_str(),
            scope: RouteScope::WORKSPACE.as_str(),
            scope_key: &context.workspace_root_uri,
            provider: context.provider.provider_id().as_str(),
            provider_version: context.document_symbols.provider_version.as_deref(),
            content_hash: Some(&context.workspace_fingerprint),
            run_id: context.reference_run_id,
            diagnostics_json: json!({
                "requested_execution_mode": execution_mode_label(),
                "actual_execution_mode": execution_mode_label(),
                "analysis_workers": context.analysis_worker_count,
                "targets_queried": route_summary.targets_queried,
                "reference_edges": summary.reference_edges,
                "reference_occurrences": summary.reference_occurrences,
                "file_fallbacks": route_summary.file_fallbacks,
                "skipped_external": route_summary.skipped_external,
            }),
        })
        .await
        .map_err(ExtractError::storage)?;
    let stale_edges_closed = context
        .store
        .close_stale_edges_for_route(CloseStaleRouteInput {
            workspace_id: context.workspace_id,
            run_id: context.reference_run_id,
            route: RouteName::RUST_REFERENCES.as_str(),
            scope: RouteScope::WORKSPACE.as_str(),
            scope_key: &context.workspace_root_uri,
            provider: context.provider.provider_id().as_str(),
        })
        .await
        .map_err(ExtractError::storage)?;
    context
        .store
        .finish_run(context.reference_run_id, "complete")
        .await
        .map_err(ExtractError::storage)?;

    summary.stale_edges_closed = stale_edges_closed as usize;
    Ok(())
}

async fn complete_call_route(
    context: &FileRelationContext,
    summary: &mut PersistenceSummary,
    route_summary: &CallRouteSummary,
) -> ExtractResult<()> {
    context
        .store
        .complete_route_status(RouteStatusCompleteInput {
            workspace_id: context.workspace_id,
            route: RouteName::RUST_CALLS.as_str(),
            scope: RouteScope::WORKSPACE.as_str(),
            scope_key: &context.workspace_root_uri,
            provider: context.provider.provider_id().as_str(),
            provider_version: context.document_symbols.provider_version.as_deref(),
            content_hash: Some(&context.workspace_fingerprint),
            run_id: context.call_run_id,
            diagnostics_json: json!({
                "requested_execution_mode": execution_mode_label(),
                "actual_execution_mode": execution_mode_label(),
                "analysis_workers": context.analysis_worker_count,
                "callable_nodes": route_summary.callable_nodes,
                "call_edges": summary.call_edges,
                "call_occurrences": summary.call_occurrences,
                "skipped_external_targets": route_summary.skipped_external_targets,
                "skipped_unresolved_targets": route_summary.skipped_unresolved_targets,
                "skipped_non_callable_prepare_items": route_summary.skipped_non_callable_prepare_items,
            }),
        })
        .await
        .map_err(ExtractError::storage)?;
    let stale_edges_closed = context
        .store
        .close_stale_edges_for_route(CloseStaleRouteInput {
            workspace_id: context.workspace_id,
            run_id: context.call_run_id,
            route: RouteName::RUST_CALLS.as_str(),
            scope: RouteScope::WORKSPACE.as_str(),
            scope_key: &context.workspace_root_uri,
            provider: context.provider.provider_id().as_str(),
        })
        .await
        .map_err(ExtractError::storage)?;
    context
        .store
        .finish_run(context.call_run_id, "complete")
        .await
        .map_err(ExtractError::storage)?;

    summary.stale_edges_closed = stale_edges_closed as usize;
    Ok(())
}

async fn fail_file_relation_routes(
    context: &FileRelationContext,
    error: &ExtractError,
) -> ExtractResult<()> {
    fail_relation_route(
        &context.store,
        context.workspace_id,
        context.reference_run_id,
        &context.workspace_root_uri,
        context.provider.provider_id().as_str(),
        RouteName::RUST_REFERENCES.as_str(),
        error,
    )
    .await?;
    fail_relation_route(
        &context.store,
        context.workspace_id,
        context.call_run_id,
        &context.workspace_root_uri,
        context.provider.provider_id().as_str(),
        RouteName::RUST_CALLS.as_str(),
        error,
    )
    .await
}

async fn fail_relation_route(
    store: &WriteHandle,
    workspace_id: i64,
    run_id: i64,
    workspace_root_uri: &str,
    provider: &str,
    route: &str,
    error: &ExtractError,
) -> ExtractResult<()> {
    store
        .fail_route_status(RouteStatusFailInput {
            workspace_id,
            route,
            scope: RouteScope::WORKSPACE.as_str(),
            scope_key: workspace_root_uri,
            provider,
            run_id,
            diagnostics_json: json!({
                "kind": error.message(),
                "error": error.to_string(),
            }),
        })
        .await
        .map_err(ExtractError::storage)?;
    store
        .finish_run(run_id, "failed")
        .await
        .map_err(ExtractError::storage)
}

async fn file_ids_for_document_symbols(
    store: &WriteHandle,
    workspace_id: i64,
    document_symbols: &DocumentSymbolBatchExtraction,
) -> ExtractResult<HashMap<String, i64>> {
    let mut file_ids = HashMap::new();
    for extraction in &document_symbols.extractions {
        let file_id = store
            .file_id(workspace_id, &extraction.source_file.uri)
            .await
            .map_err(ExtractError::storage)?
            .ok_or_else(|| {
                ExtractError::response_shape(
                    document_symbols.provider.as_str(),
                    "rust-workspace-all",
                    format!(
                        "source file {} was not persisted before threaded relation extraction",
                        extraction.source_file.uri
                    ),
                )
            })?;
        file_ids.insert(extraction.source_file.uri.clone(), file_id);
    }

    Ok(file_ids)
}

fn execution_diagnostics(
    requested_jobs: usize,
    analysis_workers: usize,
    file_work_items: usize,
) -> serde_json::Value {
    json!({
        "requested_execution_mode": execution_mode_label(),
        "actual_execution_mode": execution_mode_label(),
        "requested_jobs": requested_jobs,
        "actual_jobs": analysis_workers,
        "analysis_workers": analysis_workers,
        "scheduling_unit": "file",
        "file_work_items": file_work_items,
    })
}

fn execution_mode_label() -> &'static str {
    "file_grained_analysis_worker_pool"
}

fn effective_analysis_worker_count(config: &ThreadedWorkspaceAllConfig) -> usize {
    let split_workers = config.reference_analysis_workers() + config.call_analysis_workers();
    if split_workers == 0 {
        config.analysis_workers()
    } else {
        split_workers
    }
}

fn file_semantic_work_items(
    reference_targets: Vec<rust_analyzer_lib::ResolvedReferenceTarget>,
    call_targets: Vec<rust_analyzer_lib::ResolvedCallTarget>,
) -> Vec<rust_analyzer_lib::FileSemanticWork> {
    let mut grouped_targets = BTreeMap::new();
    for target in reference_targets {
        grouped_targets
            .entry(target.file_path.clone())
            .or_insert_with(|| (Vec::new(), Vec::new()))
            .0
            .push(target);
    }
    for target in call_targets {
        grouped_targets
            .entry(target.file_path.clone())
            .or_insert_with(|| (Vec::new(), Vec::new()))
            .1
            .push(target);
    }

    grouped_targets
        .into_iter()
        .map(
            |(file_path, (reference_targets, call_targets))| rust_analyzer_lib::FileSemanticWork {
                file_path,
                reference_targets,
                call_targets,
            },
        )
        .collect()
}

fn empty_file_relation_worker_summary(
    workspace_id: i64,
    reference_run_id: i64,
    call_run_id: i64,
) -> FileRelationWorkerSummary {
    FileRelationWorkerSummary {
        reference_persistence: empty_summary(workspace_id, reference_run_id),
        reference_route: ReferenceRouteSummary {
            targets_queried: 0,
            reference_edges: 0,
            reference_occurrences: 0,
            file_fallbacks: 0,
            skipped_external: 0,
        },
        call_persistence: empty_summary(workspace_id, call_run_id),
        call_route: CallRouteSummary {
            callable_nodes: 0,
            call_edges: 0,
            call_occurrences: 0,
            skipped_external_targets: 0,
            skipped_unresolved_targets: 0,
            skipped_non_callable_prepare_items: 0,
        },
    }
}

fn merge_file_relation_summary(
    target: &mut FileRelationWorkerSummary,
    source: &FileRelationWorkerSummary,
) {
    merge_summary(
        &mut target.reference_persistence,
        &source.reference_persistence,
    );
    target.reference_route.targets_queried += source.reference_route.targets_queried;
    target.reference_route.reference_edges += source.reference_route.reference_edges;
    target.reference_route.reference_occurrences += source.reference_route.reference_occurrences;
    target.reference_route.file_fallbacks += source.reference_route.file_fallbacks;
    target.reference_route.skipped_external += source.reference_route.skipped_external;

    merge_summary(&mut target.call_persistence, &source.call_persistence);
    target.call_route.callable_nodes += source.call_route.callable_nodes;
    target.call_route.call_edges += source.call_route.call_edges;
    target.call_route.call_occurrences += source.call_route.call_occurrences;
    target.call_route.skipped_external_targets += source.call_route.skipped_external_targets;
    target.call_route.skipped_unresolved_targets += source.call_route.skipped_unresolved_targets;
    target.call_route.skipped_non_callable_prepare_items +=
        source.call_route.skipped_non_callable_prepare_items;
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

fn symbol_count(document_symbols: &DocumentSymbolBatchExtraction) -> usize {
    document_symbols
        .extractions
        .iter()
        .map(|extraction| extraction.symbols.len())
        .sum()
}

fn workspace_fingerprint(document_symbols: &DocumentSymbolBatchExtraction) -> String {
    let mut entries = document_symbols
        .extractions
        .iter()
        .map(|extraction| {
            format!(
                "{}:{}",
                extraction.source_file.relative_path,
                extraction
                    .source_file
                    .content_hash
                    .as_deref()
                    .unwrap_or_default()
            )
        })
        .collect::<Vec<_>>();
    entries.sort();

    let mut hasher = sha2::Sha256::new();
    for entry in entries {
        sha2::Digest::update(&mut hasher, entry.as_bytes());
        sha2::Digest::update(&mut hasher, b"\n");
    }

    hex::encode(sha2::Digest::finalize(hasher))
}

fn worker_join_error(route: &str, error: JoinError) -> ExtractError {
    ExtractError::process("rust-analyzer", route, error.to_string())
}
