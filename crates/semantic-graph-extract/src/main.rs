use semantic_graph_config::{discover_config, load_config};
use semantic_graph_db_manager::{Config, WriteHandle, WriteManager};
use semantic_graph_extract::{
    ExtractError, ExtractResult,
    benchmark::{BenchmarkSummary, Stopwatch},
    cli::{
        Cli, Command, RustFileExtractions, RustFileMode, resolve_cli_database_path,
        resolve_rust_file_mode, resolve_rust_workspace_routes, symbol_key_belongs_to_file,
        validate_deleted_rust_file_request,
    },
    document_symbols::paths::{file_uri, validate_document_symbol_batch_request},
    model::{CallBatchExtraction, DocumentSymbolBatchExtraction, DocumentSymbolBatchRequest},
    persist::{ExtractionPersister, PersistenceSummary},
    providers::rust_analyzer::RustAnalyzerProvider,
    workspace_extraction::{
        ThreadedWorkspaceExtractionConfig, ThreadedWorkspaceExtractionRunner,
        WorkspaceExtractionRoutes, WorkspaceExtractionSummary,
    },
};

use clap::Parser;
use std::{
    error::Error,
    path::{Path, PathBuf},
};

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        print_error(&error);
        std::process::exit(1);
    }
}

async fn run() -> ExtractResult<()> {
    let cli = Cli::parse();
    let config = cli.config;

    match cli.command {
        Command::RustFile {
            db,
            workspace_root,
            calls,
            references,
            symbols,
            file,
        } => {
            let mode = resolve_rust_file_mode(calls, references, symbols)?;
            let document_request =
                validate_document_symbol_batch_request(DocumentSymbolBatchRequest {
                    package_path: workspace_root.clone(),
                    workspace_root,
                    file_paths: vec![file],
                })?;
            validate_rust_file_path(&document_request.file_paths[0])?;

            let workspace_root_uri = file_uri(&document_request.workspace_root)?;
            let db = resolve_cli_database_path(db, &config, &document_request.workspace_root)?;
            let store = start_writer(db, &config, &document_request.workspace_root).await?;

            let provider = RustAnalyzerProvider::new();
            let extractions =
                extract_rust_file_with_single_worker(&provider, document_request, mode).await?;
            let summary =
                persist_rust_file_extractions(&store, &workspace_root_uri, mode, &extractions)
                    .await?;
            shutdown_writer(&store).await?;

            print_rust_file_summary(mode, &summary);
        }
        Command::RustFileDeleted {
            db,
            workspace_root,
            file,
        } => {
            let (workspace_root, deleted_file_uri, relative_path) =
                validate_deleted_rust_file_request(workspace_root, &file)?;
            let workspace_root_uri = file_uri(&workspace_root)?;
            let db = resolve_cli_database_path(db, &config, &workspace_root)?;
            let store = start_writer(db, &config, &workspace_root).await?;

            let summary = ExtractionPersister
                .mark_deleted_rust_file_stale(&store, &workspace_root_uri, &deleted_file_uri)
                .await?;
            shutdown_writer(&store).await?;

            print_rust_file_deleted_summary(&relative_path, &summary);
        }
        Command::RustCrate {
            db,
            workspace_root,
            analysis_workers,
            calls,
            references,
            symbols,
            package_path,
        } => {
            let routes = resolve_rust_workspace_routes(calls, references, symbols);
            let provider = RustAnalyzerProvider::new();

            let discovery_timer = Stopwatch::start_new();
            let file_paths = provider.discover_rust_source_files(&workspace_root, &package_path)?;
            let discovery_elapsed = discovery_timer.elapsed();

            let document_request =
                validate_document_symbol_batch_request(DocumentSymbolBatchRequest {
                    workspace_root,
                    package_path,
                    file_paths,
                })?;
            let summary = run_threaded_rust_route_batch(
                &config,
                db,
                &provider,
                document_request,
                routes,
                analysis_workers,
                Some(("discovery", discovery_elapsed)),
            )
            .await?;

            print_rust_route_batch_summary("crate", routes, &summary);
            print_benchmark_summary(&summary.benchmark);
        }
        Command::RustWorkspace {
            db,
            workspace_root,
            analysis_workers,
            calls,
            references,
            symbols,
        } => {
            let routes = resolve_rust_workspace_routes(calls, references, symbols);
            let provider = RustAnalyzerProvider::new();

            let discovery_timer = Stopwatch::start_new();
            let file_paths = provider.discover_rust_workspace_source_files(&workspace_root)?;
            let discovery_elapsed = discovery_timer.elapsed();

            let document_request =
                validate_document_symbol_batch_request(DocumentSymbolBatchRequest {
                    package_path: workspace_root.clone(),
                    workspace_root,
                    file_paths,
                })?;
            let summary = run_threaded_rust_route_batch(
                &config,
                db,
                &provider,
                document_request,
                routes,
                analysis_workers,
                Some(("discovery", discovery_elapsed)),
            )
            .await?;

            print_rust_route_batch_summary("workspace", routes, &summary);
            print_benchmark_summary(&summary.benchmark);
        }
    }

    Ok(())
}

fn print_error(error: &dyn Error) {
    eprintln!("{error}");

    let mut source = error.source();
    while let Some(error) = source {
        eprintln!("caused by: {error}");
        source = error.source();
    }
}

fn validate_rust_file_path(file_path: &Path) -> ExtractResult<()> {
    if file_path.is_file() {
        return Ok(());
    }

    Err(ExtractError::invalid_path(
        file_path,
        PathBuf::new(),
        "rust-file requires the path to a single file",
    ))
}

async fn extract_rust_file_with_single_worker(
    provider: &RustAnalyzerProvider,
    document_request: DocumentSymbolBatchRequest,
    mode: RustFileMode,
) -> ExtractResult<RustFileExtractions> {
    let worker = rust_analyzer_lib::AnalysisWorker::start(&document_request.workspace_root)
        .map_err(|source| {
            ExtractError::rust_analyzer_lib("start single rust-file worker", source)
        })?;
    let result = extract_rust_file_with_worker(provider, &worker, document_request, mode).await;
    let shutdown_result = worker.shutdown().await.map_err(|source| {
        ExtractError::rust_analyzer_lib("shutdown single rust-file worker", source)
    });

    match (result, shutdown_result) {
        (Ok(extractions), Ok(())) => Ok(extractions),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
    }
}

async fn extract_rust_file_with_worker(
    provider: &RustAnalyzerProvider,
    worker: &rust_analyzer_lib::AnalysisWorkerHandle,
    document_request: DocumentSymbolBatchRequest,
    mode: RustFileMode,
) -> ExtractResult<RustFileExtractions> {
    let file_scope_key = file_uri(&document_request.file_paths[0])?;
    let document_symbols =
        document_symbols_with_worker(provider, worker, document_request.clone()).await?;

    if !mode.includes_references() && !mode.includes_calls() {
        return Ok(RustFileExtractions {
            file_scope_key,
            document_symbols,
            references: None,
            calls: None,
        });
    }

    let (relation_document_request, relation_document_symbols) =
        rust_file_relation_document_symbols(provider, worker, &document_request).await?;

    let reference_targets = if mode.includes_references() {
        provider.reference_targets_for_document_symbols(
            &relation_document_request,
            &relation_document_symbols,
        )?
    } else if mode.includes_calls() {
        provider.reference_targets_for_document_symbols(&document_request, &document_symbols)?
    } else {
        Vec::new()
    };
    let reference_target_count = reference_targets.len();
    let reference_result = worker
        .file_semantic_work(rust_analyzer_lib::FileSemanticWork {
            file_path: document_request.file_paths[0].clone(),
            reference_targets,
            call_targets: Vec::new(),
        })
        .await
        .map_err(|source| {
            ExtractError::rust_analyzer_lib("rust-analyzer-lib file_semantic_work", source)
        })?;

    let references = if mode.includes_references() {
        let reference_sets = reference_sets_for_file_relations(
            reference_result.reference_sets.clone(),
            &document_request.file_paths[0],
        );
        Some(provider.map_reference_sets(
            &relation_document_request,
            relation_document_symbols.clone(),
            reference_sets,
            reference_target_count,
        )?)
    } else {
        None
    };
    let calls = if mode.includes_calls() {
        let inbound_caller_files = caller_files_referencing_file(
            &reference_result.reference_sets,
            &document_request.file_paths[0],
        );
        let call_targets = call_targets_for_file_relations(
            provider.call_targets_for_document_symbols(
                &relation_document_request,
                &relation_document_symbols,
            )?,
            &document_request.file_paths[0],
            &inbound_caller_files,
        );
        let call_target_count = call_targets.len();
        let call_result = worker
            .file_semantic_work(rust_analyzer_lib::FileSemanticWork {
                file_path: document_request.file_paths[0].clone(),
                reference_targets: Vec::new(),
                call_targets,
            })
            .await
            .map_err(|source| {
                ExtractError::rust_analyzer_lib("rust-analyzer-lib file_semantic_work", source)
            })?;
        let calls = provider.map_call_sets(
            &relation_document_request,
            relation_document_symbols,
            call_result.call_sets,
            call_target_count,
        )?;
        Some(call_extraction_for_file_relations(calls, &file_scope_key))
    } else {
        None
    };

    Ok(RustFileExtractions {
        file_scope_key,
        document_symbols,
        references,
        calls,
    })
}

async fn document_symbols_with_worker(
    provider: &RustAnalyzerProvider,
    worker: &rust_analyzer_lib::AnalysisWorkerHandle,
    document_request: DocumentSymbolBatchRequest,
) -> ExtractResult<DocumentSymbolBatchExtraction> {
    let document_symbol_items = worker
        .document_symbols_for_files(document_request.file_paths.clone())
        .await
        .map_err(|source| {
            ExtractError::rust_analyzer_lib("rust-analyzer-lib document_symbols_for_files", source)
        })?;

    provider.map_document_symbol_items(document_request, document_symbol_items)
}

async fn rust_file_relation_document_symbols(
    provider: &RustAnalyzerProvider,
    worker: &rust_analyzer_lib::AnalysisWorkerHandle,
    document_request: &DocumentSymbolBatchRequest,
) -> ExtractResult<(DocumentSymbolBatchRequest, DocumentSymbolBatchExtraction)> {
    let mut file_paths =
        provider.discover_rust_workspace_source_files(&document_request.workspace_root)?;
    file_paths.push(document_request.file_paths[0].clone());

    let relation_document_request =
        validate_document_symbol_batch_request(DocumentSymbolBatchRequest {
            package_path: document_request.workspace_root.clone(),
            workspace_root: document_request.workspace_root.clone(),
            file_paths,
        })?;
    let relation_document_symbols =
        document_symbols_with_worker(provider, worker, relation_document_request.clone()).await?;

    Ok((relation_document_request, relation_document_symbols))
}

fn reference_sets_for_file_relations(
    mut reference_sets: Vec<rust_analyzer_lib::ResolvedReferenceSet>,
    file_path: &Path,
) -> Vec<rust_analyzer_lib::ResolvedReferenceSet> {
    for reference_set in &mut reference_sets {
        let target_is_in_file = reference_set.target_file_path == file_path;
        reference_set
            .references
            .retain(|location| target_is_in_file || location.file_path == file_path);
    }

    reference_sets
        .into_iter()
        .filter(|reference_set| !reference_set.references.is_empty())
        .collect()
}

fn caller_files_referencing_file(
    reference_sets: &[rust_analyzer_lib::ResolvedReferenceSet],
    file_path: &Path,
) -> Vec<PathBuf> {
    let mut caller_files = Vec::new();
    for reference_set in reference_sets {
        if reference_set.target_file_path != file_path {
            continue;
        }

        for location in &reference_set.references {
            if location.file_path == file_path || caller_files.contains(&location.file_path) {
                continue;
            }

            caller_files.push(location.file_path.clone());
        }
    }

    caller_files
}

fn call_targets_for_file_relations(
    call_targets: Vec<rust_analyzer_lib::ResolvedCallTarget>,
    file_path: &Path,
    inbound_caller_files: &[PathBuf],
) -> Vec<rust_analyzer_lib::ResolvedCallTarget> {
    let mut selected_targets = Vec::new();
    for target in call_targets {
        if target.file_path != file_path && !inbound_caller_files.contains(&target.file_path) {
            continue;
        }
        if selected_targets.contains(&target) {
            continue;
        }

        selected_targets.push(target);
    }

    selected_targets
}

fn call_extraction_for_file_relations(
    mut extraction: CallBatchExtraction,
    file_scope_key: &str,
) -> CallBatchExtraction {
    extraction.calls = extraction
        .calls
        .into_iter()
        .filter_map(|mut call| {
            let callee_is_in_file =
                symbol_key_belongs_to_file(&call.callee_symbol_key, file_scope_key);
            call.occurrences
                .retain(|occurrence| occurrence.file_uri == file_scope_key || callee_is_in_file);
            if call.occurrences.is_empty() {
                None
            } else {
                Some(call)
            }
        })
        .collect();
    extraction.summary.call_edges = extraction.calls.len();
    extraction.summary.call_occurrences = extraction
        .calls
        .iter()
        .map(|call| call.occurrences.len())
        .sum();

    extraction
}

async fn persist_rust_file_extractions(
    store: &WriteHandle,
    workspace_root_uri: &str,
    mode: RustFileMode,
    extractions: &RustFileExtractions,
) -> ExtractResult<PersistenceSummary> {
    let mut summary = None;

    if mode.includes_symbols() {
        let document_summary = ExtractionPersister
            .persist_document_symbol_batch(store, workspace_root_uri, &extractions.document_symbols)
            .await?;
        merge_optional_summary(&mut summary, document_summary);
    }

    if let Some(references) = &extractions.references {
        let reference_summary = ExtractionPersister
            .persist_reference_file_batch_for_file(
                store,
                workspace_root_uri,
                &extractions.file_scope_key,
                references,
            )
            .await?;
        merge_optional_summary(&mut summary, reference_summary);
    }

    if let Some(calls) = &extractions.calls {
        let call_summary = ExtractionPersister
            .persist_call_file_batch_for_file(
                store,
                workspace_root_uri,
                &extractions.file_scope_key,
                calls,
            )
            .await?;
        merge_optional_summary(&mut summary, call_summary);
    }

    summary.ok_or_else(|| {
        ExtractError::response_shape(
            "rust-analyzer",
            "rust-file",
            "rust-file extraction produced no persistence work",
        )
    })
}

fn merge_optional_summary(target: &mut Option<PersistenceSummary>, source: PersistenceSummary) {
    match target {
        Some(target) => {
            target.run_id = source.run_id;
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
        None => {
            *target = Some(source);
        }
    }
}

fn print_rust_file_summary(mode: RustFileMode, summary: &PersistenceSummary) {
    let contains_edges = summary
        .edges
        .saturating_sub(summary.reference_edges + summary.call_edges);
    println!(
        "mode={} workspace={} last_run={} files={} nodes={} contains_edges={} references_edges={} reference_occurrences={} calls_edges={} call_occurrences={} evidence={} routes_complete={} stale_nodes_closed={} stale_edges_closed={}",
        mode.label(),
        summary.workspace_id,
        summary.run_id,
        summary.files,
        summary.nodes,
        contains_edges,
        summary.reference_edges,
        summary.reference_occurrences,
        summary.call_edges,
        summary.call_occurrences,
        summary.evidence,
        summary.routes_complete,
        summary.stale_nodes_closed,
        summary.stale_edges_closed
    );
}

fn print_rust_file_deleted_summary(relative_path: &str, summary: &PersistenceSummary) {
    println!(
        "mode=deleted file={} workspace={} run={} routes_complete={} stale_nodes_closed={} stale_edges_closed={}",
        relative_path,
        summary.workspace_id,
        summary.run_id,
        summary.routes_complete,
        summary.stale_nodes_closed,
        summary.stale_edges_closed
    );
}

async fn run_threaded_rust_route_batch(
    config: &Option<PathBuf>,
    db: Option<PathBuf>,
    provider: &RustAnalyzerProvider,
    document_request: DocumentSymbolBatchRequest,
    routes: WorkspaceExtractionRoutes,
    analysis_workers: Option<usize>,
    discovery_metric: Option<(&str, std::time::Duration)>,
) -> ExtractResult<WorkspaceExtractionSummary> {
    let total_timer = Stopwatch::start_new();
    let mut benchmark = BenchmarkSummary::new();

    if let Some((name, elapsed)) = discovery_metric {
        benchmark.insert_duration_ms(name, elapsed);
    }
    benchmark.insert_count("files_discovered", document_request.file_paths.len());

    let writer_ready_timer = Stopwatch::start_new();
    let db = resolve_cli_database_path(db, config, &document_request.workspace_root)?;
    let store = start_writer(db, config, &document_request.workspace_root).await?;
    benchmark.insert_duration_ms("writer_ready", writer_ready_timer.elapsed());

    let extractor_plan_timer = Stopwatch::start_new();
    let analysis_worker_count = resolve_threaded_route_analysis_workers(
        config,
        &document_request.workspace_root,
        analysis_workers,
    )?;
    let reference_jobs = if routes.includes_references() {
        analysis_worker_count
    } else {
        0
    };
    let call_jobs = if routes.includes_calls() {
        analysis_worker_count
    } else {
        0
    };
    benchmark.insert_duration_ms("extractor_plan", extractor_plan_timer.elapsed());
    benchmark.insert_label("mode", "threaded");
    benchmark.insert_label("routes", routes.label());
    benchmark.insert_count("analysis_workers", analysis_worker_count);
    benchmark.insert_count("reference_jobs", reference_jobs);
    benchmark.insert_count("call_jobs", call_jobs);

    let threaded_timer = Stopwatch::start_new();
    let summary = ThreadedWorkspaceExtractionRunner::run(
        &store,
        provider,
        document_request,
        ThreadedWorkspaceExtractionConfig::with_routes(
            reference_jobs,
            call_jobs,
            analysis_worker_count,
            0,
            0,
            routes,
        ),
    )
    .await;
    benchmark.insert_duration_ms("threaded_runner", threaded_timer.elapsed());

    let writer_shutdown_timer = Stopwatch::start_new();
    shutdown_writer(&store).await?;
    benchmark.insert_duration_ms("writer_shutdown", writer_shutdown_timer.elapsed());
    let mut summary = summary?;
    benchmark.extend_from(&summary.benchmark);
    benchmark.insert_duration_ms("total", total_timer.elapsed());
    summary.benchmark = benchmark;

    Ok(summary)
}

fn resolve_threaded_route_analysis_workers(
    config: &Option<PathBuf>,
    workspace_root: &Path,
    analysis_workers: Option<usize>,
) -> ExtractResult<usize> {
    let extractor_config = resolve_cli_extractor_config(config, workspace_root)?;
    let analysis_workers = analysis_workers
        .or_else(|| extractor_config.analysis_workers())
        .unwrap_or(1);
    validate_single_route_jobs("--analysis-workers", analysis_workers)?;
    Ok(analysis_workers)
}

fn print_rust_route_batch_summary(
    scope: &str,
    routes: WorkspaceExtractionRoutes,
    summary: &WorkspaceExtractionSummary,
) {
    let contains_edges = summary.document_summary.edges;
    println!(
        "scope={} mode={} workspace={} last_run={} files={} nodes={} contains_edges={} references_edges={} reference_occurrences={} calls_edges={} call_occurrences={} evidence={} routes_complete={} stale_nodes_closed={} stale_edges_closed={}",
        scope,
        routes.label(),
        summary.document_summary.workspace_id,
        selected_route_last_run(routes, summary),
        summary.document_summary.files,
        summary.document_summary.nodes,
        contains_edges,
        summary.reference_summary.reference_edges,
        summary.reference_summary.reference_occurrences,
        summary.call_summary.call_edges,
        summary.call_summary.call_occurrences,
        summary.document_summary.evidence
            + summary.reference_summary.evidence
            + summary.call_summary.evidence,
        summary.document_summary.routes_complete
            + summary.reference_summary.routes_complete
            + summary.call_summary.routes_complete,
        summary.document_summary.stale_nodes_closed,
        summary.document_summary.stale_edges_closed
            + summary.reference_summary.stale_edges_closed
            + summary.call_summary.stale_edges_closed
    );
}

fn selected_route_last_run(
    routes: WorkspaceExtractionRoutes,
    summary: &WorkspaceExtractionSummary,
) -> i64 {
    [
        routes
            .includes_symbols()
            .then_some(summary.document_summary.run_id),
        routes
            .includes_references()
            .then_some(summary.reference_summary.run_id),
        routes
            .includes_calls()
            .then_some(summary.call_summary.run_id),
    ]
    .into_iter()
    .flatten()
    .max()
    .unwrap_or(0)
}

fn resolve_cli_extractor_config(
    config: &Option<PathBuf>,
    workspace_root: &Path,
) -> ExtractResult<semantic_graph_config::ExtractorConfig> {
    let config_path = match config {
        Some(path) => Some(path.clone()),
        None => discover_config(workspace_root).map_err(ExtractError::config)?,
    };

    let Some(config_path) = config_path else {
        return Ok(semantic_graph_config::ExtractorConfig::default());
    };

    let config = load_config(config_path).map_err(ExtractError::config)?;
    Ok(config.extractor().clone())
}

fn validate_single_route_jobs(name: &str, value: usize) -> ExtractResult<()> {
    if value == 0 {
        return Err(invalid_worker_split(&format!(
            "{name} must be greater than zero"
        )));
    }

    Ok(())
}

fn invalid_worker_split(message: &str) -> ExtractError {
    ExtractError::response_shape("rust-analyzer", "rust-workspace", message)
}

fn print_benchmark_summary(summary: &BenchmarkSummary) {
    for line in summary.lines() {
        println!("{line}");
    }
}

async fn start_writer(
    db: PathBuf,
    config: &Option<PathBuf>,
    workspace_root: &Path,
) -> ExtractResult<WriteHandle> {
    let writer_config = resolve_cli_writer_config(config, workspace_root)?;
    let writer = WriteManager::start_with_config(db, writer_config)
        .await
        .map_err(ExtractError::storage)?;
    writer.migrate().await.map_err(ExtractError::storage)?;
    Ok(writer)
}

async fn shutdown_writer(writer: &WriteHandle) -> ExtractResult<()> {
    writer
        .shutdown()
        .await
        .map(|_| ())
        .map_err(ExtractError::storage)
}

fn resolve_cli_writer_config(
    config: &Option<PathBuf>,
    workspace_root: &Path,
) -> ExtractResult<Config> {
    let config_path = match config {
        Some(path) => Some(path.clone()),
        None => discover_config(workspace_root).map_err(ExtractError::config)?,
    };

    let Some(config_path) = config_path else {
        return Ok(Config::default());
    };

    let config = load_config(config_path).map_err(ExtractError::config)?;
    Ok(Config::from(config.writer()))
}
