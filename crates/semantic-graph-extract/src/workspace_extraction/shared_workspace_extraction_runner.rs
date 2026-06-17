use crate::{
    ExtractError, ExtractResult,
    benchmark::{BenchmarkSummary, Stopwatch},
    document_symbols::paths::file_uri,
    model::{
        CallRouteSummary, DocumentSymbolBatchExtraction, DocumentSymbolBatchRequest,
        ReferenceRouteSummary,
    },
    persist::{ExtractionPersister, PersistenceSummary},
    providers::rust_analyzer::RustAnalyzerProvider,
    workspace_extraction::{ThreadedWorkspaceExtractionConfig, WorkspaceExtractionSummary},
};

use semantic_graph_db_manager::WriteHandle;

use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{sync::Mutex, task::JoinError};

type FileRelationWorkerMetric = (usize, usize, usize, usize, Duration);
type FileRelationWorkerResult = (
    Vec<rust_analyzer_lib::FileSemanticResult>,
    FileRelationWorkerMetric,
);
type FileRelationWorkerJoinHandle =
    tokio::task::JoinHandle<ExtractResult<FileRelationWorkerResult>>;

pub struct SharedWorkspaceExtractionRunner;

impl SharedWorkspaceExtractionRunner {
    pub async fn run(
        store: &WriteHandle,
        provider: &RustAnalyzerProvider,
        document_request: DocumentSymbolBatchRequest,
        config: ThreadedWorkspaceExtractionConfig,
        close_stale_document_symbols: bool,
    ) -> ExtractResult<WorkspaceExtractionSummary> {
        let total_timer = Stopwatch::start_new();
        let mut benchmark = BenchmarkSummary::new();
        let analysis_worker_count = effective_analysis_worker_count(&config);
        let routes = config.routes();
        benchmark.insert_label("shared_workspace.execution_mode", execution_mode_label());
        benchmark.insert_label("shared_workspace.routes", routes.label());
        benchmark.insert_count("shared_workspace.analysis_workers", analysis_worker_count);
        benchmark.insert_count(
            "shared_workspace.input_files",
            document_request.file_paths.len(),
        );
        benchmark.insert_count(
            "shared_workspace.extract_symbols",
            usize::from(routes.includes_symbols()),
        );
        benchmark.insert_count(
            "shared_workspace.extract_references",
            usize::from(routes.includes_references()),
        );
        benchmark.insert_count(
            "shared_workspace.extract_calls",
            usize::from(routes.includes_calls()),
        );
        benchmark.insert_count(
            "shared_workspace.close_stale_document_symbols",
            usize::from(close_stale_document_symbols),
        );

        let workspace_root_uri_timer = Stopwatch::start_new();
        let workspace_root_uri = file_uri(&document_request.workspace_root)?;
        benchmark.insert_duration_ms(
            "shared_workspace.workspace_root_uri",
            workspace_root_uri_timer.elapsed(),
        );

        let analysis_pool_timer = Stopwatch::start_new();
        let analysis_pool = rust_analyzer_lib::SharedAnalysisWorkerPool::start(
            &document_request.workspace_root,
            analysis_worker_count,
        )
        .map_err(|source| {
            ExtractError::rust_analyzer_lib("start shared analysis snapshot pool", source)
        })?;
        benchmark.insert_duration_ms(
            "shared_workspace.analysis_pool_start",
            analysis_pool_timer.elapsed(),
        );
        benchmark.insert_count(
            "shared_workspace.analysis_pool_workers",
            analysis_pool.worker_count(),
        );

        let document_symbols_query_timer = Stopwatch::start_new();
        let document_symbol_items = analysis_pool
            .document_symbols_for_files(document_request.file_paths.clone())
            .await
            .map_err(|source| {
                ExtractError::rust_analyzer_lib(
                    "rust-analyzer-lib shared document_symbols_for_files",
                    source,
                )
            })?;
        benchmark.insert_duration_ms(
            "shared_workspace.document_symbols_query",
            document_symbols_query_timer.elapsed(),
        );

        let document_symbols_map_timer = Stopwatch::start_new();
        let document_symbols =
            provider.map_document_symbol_items(document_request.clone(), document_symbol_items)?;
        benchmark.insert_count(
            "shared_workspace.document_files",
            document_symbols.extractions.len(),
        );
        benchmark.insert_count(
            "shared_workspace.document_symbols",
            symbol_count(&document_symbols),
        );
        benchmark.insert_duration_ms(
            "shared_workspace.document_symbols_map",
            document_symbols_map_timer.elapsed(),
        );

        let document_symbols_persist_timer = Stopwatch::start_new();
        let document_summary = if routes.includes_symbols() {
            ExtractionPersister
                .persist_document_symbol_batch_with_write_batch(
                    store,
                    &workspace_root_uri,
                    &document_symbols,
                    close_stale_document_symbols,
                )
                .await?
        } else {
            let workspace_id = existing_workspace_id(
                store,
                &workspace_root_uri,
                provider.provider_id().as_str(),
                "rust-workspace route-only extraction",
            )
            .await?;
            empty_summary(workspace_id, 0)
        };
        benchmark.insert_duration_ms(
            "shared_workspace.document_symbols_persist",
            document_symbols_persist_timer.elapsed(),
        );
        if routes.includes_symbols() {
            benchmark.insert_label(
                "shared_workspace.document_symbols_write_mode",
                "document_symbol_write_batch",
            );
        }

        let targets_timer = Stopwatch::start_new();
        let reference_targets = if routes.includes_references() {
            provider.reference_targets_for_document_symbols(&document_request, &document_symbols)?
        } else {
            Vec::new()
        };
        let call_targets = if routes.includes_calls() {
            provider.call_targets_for_document_symbols(&document_request, &document_symbols)?
        } else {
            Vec::new()
        };
        let reference_target_count = reference_targets.len();
        let call_target_count = call_targets.len();
        benchmark.insert_count("shared_workspace.reference_targets", reference_target_count);
        benchmark.insert_count("shared_workspace.call_targets", call_target_count);
        benchmark.insert_duration_ms("shared_workspace.targets_build", targets_timer.elapsed());

        let file_work_timer = Stopwatch::start_new();
        let file_work_items = file_semantic_work_items(reference_targets, call_targets);
        benchmark.insert_count("shared_workspace.file_work_items", file_work_items.len());
        benchmark.insert_duration_ms(
            "shared_workspace.file_work_build",
            file_work_timer.elapsed(),
        );

        let mut reference_summary = empty_summary(document_summary.workspace_id, 0);
        let mut call_summary = empty_summary(document_summary.workspace_id, 0);
        let mut reference_route_summary = empty_reference_route_summary();
        let mut call_route_summary = empty_call_route_summary();

        if routes.includes_relations() {
            let relations_timer = Stopwatch::start_new();
            let (file_results, worker_metrics) =
                run_file_relation_workers(file_work_items, analysis_pool.worker_handles()).await?;
            benchmark
                .insert_duration_ms("shared_workspace.file_relations", relations_timer.elapsed());
            insert_file_relation_worker_metrics(&mut benchmark, &worker_metrics);

            if routes.includes_references() {
                let reference_sets = file_results
                    .iter()
                    .flat_map(|result| result.reference_sets.clone())
                    .collect::<Vec<_>>();
                let reference_map_timer = Stopwatch::start_new();
                let reference_extraction = provider.map_reference_sets(
                    &document_request,
                    document_symbols.clone(),
                    reference_sets,
                    reference_target_count,
                )?;
                reference_route_summary = reference_extraction.summary.clone();
                benchmark.insert_duration_ms(
                    "shared_workspace.references_map",
                    reference_map_timer.elapsed(),
                );

                let reference_persist_timer = Stopwatch::start_new();
                reference_summary = ExtractionPersister
                    .persist_reference_batch_with_route_write_batch(
                        store,
                        &workspace_root_uri,
                        &reference_extraction,
                    )
                    .await?;
                benchmark.insert_label(
                    "shared_workspace.references_write_mode",
                    "route_write_batch",
                );
                benchmark.insert_duration_ms(
                    "shared_workspace.references_persist",
                    reference_persist_timer.elapsed(),
                );
            }

            if routes.includes_calls() {
                let call_sets = file_results
                    .into_iter()
                    .flat_map(|result| result.call_sets)
                    .collect::<Vec<_>>();
                let call_map_timer = Stopwatch::start_new();
                let call_extraction = provider.map_call_sets(
                    &document_request,
                    document_symbols,
                    call_sets,
                    call_target_count,
                )?;
                call_route_summary = call_extraction.summary.clone();
                benchmark
                    .insert_duration_ms("shared_workspace.calls_map", call_map_timer.elapsed());

                let call_persist_timer = Stopwatch::start_new();
                call_summary = ExtractionPersister
                    .persist_call_batch_with_route_write_batch(
                        store,
                        &workspace_root_uri,
                        &call_extraction,
                    )
                    .await?;
                benchmark.insert_label("shared_workspace.calls_write_mode", "route_write_batch");
                benchmark.insert_duration_ms(
                    "shared_workspace.calls_persist",
                    call_persist_timer.elapsed(),
                );
            }
        } else {
            benchmark.insert_duration_ms(
                "shared_workspace.file_relations",
                std::time::Duration::from_millis(0),
            );
        }

        let analysis_pool_shutdown_timer = Stopwatch::start_new();
        analysis_pool.shutdown().await.map_err(|source| {
            ExtractError::rust_analyzer_lib("shutdown shared analysis snapshot pool", source)
        })?;
        benchmark.insert_duration_ms(
            "shared_workspace.analysis_pool_shutdown",
            analysis_pool_shutdown_timer.elapsed(),
        );

        benchmark.insert_duration_ms("shared_workspace.total", total_timer.elapsed());

        Ok(WorkspaceExtractionSummary {
            benchmark,
            document_summary,
            reference_summary,
            call_summary,
            reference_route_summary,
            call_route_summary,
        })
    }
}

async fn run_file_relation_workers(
    file_work_items: Vec<rust_analyzer_lib::FileSemanticWork>,
    worker_handles: Vec<rust_analyzer_lib::SharedAnalysisWorkerHandle>,
) -> ExtractResult<(
    Vec<rust_analyzer_lib::FileSemanticResult>,
    Vec<FileRelationWorkerMetric>,
)> {
    let queue = Arc::new(Mutex::new(VecDeque::from(file_work_items)));
    let failed = Arc::new(AtomicBool::new(false));
    let mut handles = Vec::with_capacity(worker_handles.len());

    for (worker_index, analysis_worker) in worker_handles.into_iter().enumerate() {
        let worker_queue = Arc::clone(&queue);
        let worker_failed = Arc::clone(&failed);
        let worker_failed_for_result = Arc::clone(&worker_failed);
        handles.push(tokio::spawn(async move {
            let result =
                file_relation_worker(worker_index, analysis_worker, worker_queue, worker_failed)
                    .await;
            if result.is_err() {
                worker_failed_for_result.store(true, Ordering::SeqCst);
            }
            result
        }));
    }

    collect_file_relation_workers(handles).await
}

async fn file_relation_worker(
    worker_index: usize,
    analysis_worker: rust_analyzer_lib::SharedAnalysisWorkerHandle,
    queue: Arc<Mutex<VecDeque<rust_analyzer_lib::FileSemanticWork>>>,
    failed: Arc<AtomicBool>,
) -> ExtractResult<(
    Vec<rust_analyzer_lib::FileSemanticResult>,
    FileRelationWorkerMetric,
)> {
    let timer = Stopwatch::start_new();
    let mut file_results = Vec::new();
    let mut file_count = 0;
    let mut reference_target_count = 0;
    let mut call_target_count = 0;

    loop {
        if failed.load(Ordering::SeqCst) {
            return Ok((
                file_results,
                (
                    worker_index,
                    file_count,
                    reference_target_count,
                    call_target_count,
                    timer.elapsed(),
                ),
            ));
        }

        let work = {
            let mut queue = queue.lock().await;
            queue.pop_front()
        };
        let Some(work) = work else {
            return Ok((
                file_results,
                (
                    worker_index,
                    file_count,
                    reference_target_count,
                    call_target_count,
                    timer.elapsed(),
                ),
            ));
        };

        let work_reference_targets = work.reference_targets.len();
        let work_call_targets = work.call_targets.len();
        let file_result = analysis_worker
            .file_semantic_work(work)
            .await
            .map_err(|source| {
                ExtractError::rust_analyzer_lib(
                    "rust-analyzer-lib shared file_semantic_work",
                    source,
                )
            })?;
        file_count += 1;
        reference_target_count += work_reference_targets;
        call_target_count += work_call_targets;
        file_results.push(file_result);
    }
}

async fn collect_file_relation_workers(
    handles: Vec<FileRelationWorkerJoinHandle>,
) -> ExtractResult<(
    Vec<rust_analyzer_lib::FileSemanticResult>,
    Vec<FileRelationWorkerMetric>,
)> {
    let mut file_results = Vec::new();
    let mut worker_metrics = Vec::new();
    let mut first_error = None;

    for handle in handles {
        match handle.await {
            Ok(Ok((mut worker_results, worker_metric))) => {
                file_results.append(&mut worker_results);
                worker_metrics.push(worker_metric);
            }
            Ok(Err(error)) => {
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
            Err(error) => {
                if first_error.is_none() {
                    first_error = Some(worker_join_error("rust.workspace.file_relations", error));
                }
            }
        }
    }

    match first_error {
        Some(error) => Err(error),
        None => {
            worker_metrics.sort_by_key(|(worker_index, _, _, _, _)| *worker_index);
            Ok((file_results, worker_metrics))
        }
    }
}

fn insert_file_relation_worker_metrics(
    benchmark: &mut BenchmarkSummary,
    worker_metrics: &[FileRelationWorkerMetric],
) {
    benchmark.insert_count(
        "shared_workspace.file_relation_workers",
        worker_metrics.len(),
    );
    benchmark.insert_count(
        "shared_workspace.file_relation_active_workers",
        worker_metrics
            .iter()
            .filter(
                |(_worker_index, file_count, _reference_targets, _call_targets, _elapsed)| {
                    *file_count > 0
                },
            )
            .count(),
    );

    for (worker_index, file_count, reference_targets, call_targets, elapsed) in worker_metrics {
        let prefix = format!("shared_workspace.worker.{worker_index}");
        benchmark.insert_count(&format!("{prefix}.files"), *file_count);
        benchmark.insert_count(&format!("{prefix}.reference_targets"), *reference_targets);
        benchmark.insert_count(&format!("{prefix}.call_targets"), *call_targets);
        benchmark.insert_duration_ms(&format!("{prefix}.elapsed"), *elapsed);
    }
}

async fn existing_workspace_id(
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
                    "workspace {workspace_root_uri} is missing; run rust-workspace --symbols first"
                ),
            )
        })
}

fn execution_mode_label() -> &'static str {
    "shared_analysis_snapshot"
}

fn effective_analysis_worker_count(config: &ThreadedWorkspaceExtractionConfig) -> usize {
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
    let mut grouped_targets = std::collections::BTreeMap::new();
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

fn empty_reference_route_summary() -> ReferenceRouteSummary {
    ReferenceRouteSummary {
        targets_queried: 0,
        reference_edges: 0,
        reference_occurrences: 0,
        file_fallbacks: 0,
        skipped_external: 0,
    }
}

fn empty_call_route_summary() -> CallRouteSummary {
    CallRouteSummary {
        callable_nodes: 0,
        call_edges: 0,
        call_occurrences: 0,
        skipped_external_targets: 0,
        skipped_unresolved_targets: 0,
        skipped_non_callable_prepare_items: 0,
    }
}

fn symbol_count(document_symbols: &DocumentSymbolBatchExtraction) -> usize {
    document_symbols
        .extractions
        .iter()
        .map(|extraction| extraction.symbols.len())
        .sum()
}

fn worker_join_error(route: &str, error: JoinError) -> ExtractError {
    ExtractError::process("rust-analyzer", route, error.to_string())
}
