use semantic_graph_config::{discover_config, load_config};
use semantic_graph_db_manager::{Config, WriteHandle, WriteManager};
use semantic_graph_extract::{
    ExtractError, ExtractResult,
    benchmark::{BenchmarkSummary, Stopwatch},
    cli::{
        CSharpFileExtractions, CSharpFileMode, Cli, Command, ResolvedCSharpExtractorPlan,
        RustFileExtractions, RustFileMode, normalize_lexical_path, resolve_cli_database_path,
        resolve_cli_fts_analysis_workers, resolve_cli_fts_database_path,
        resolve_csharp_extractor_plan, resolve_csharp_file_mode, resolve_csharp_workspace_routes,
        resolve_rust_file_mode, resolve_rust_workspace_routes, symbol_key_belongs_to_file,
        validate_deleted_rust_file_request,
    },
    document_symbols::paths::{file_uri, validate_document_symbol_batch_request},
    fts::{FtsExtractionOptions, FtsExtractionRunner, FtsExtractionSummary},
    model::{
        CallBatchExtraction, CallRouteSummary, DocumentSymbolBatchExtraction,
        DocumentSymbolBatchRequest, GraphLanguage, ProviderId, ReferenceRouteSummary, RouteName,
    },
    persist::{ExtractionPersister, PersistenceSummary},
    providers::csharp_ls::CSharpLsProvider,
    providers::rust_analyzer::RustAnalyzerProvider,
    workspace_extraction::{
        CSharpRouteBatchContext, CSharpRouteBatchScope, SharedWorkspaceExtractionRunner,
        ThreadedWorkspaceExtractionConfig, ThreadedWorkspaceExtractionRunner,
        WorkspaceExtractionRoutes, WorkspaceExtractionSummary, call_route_summary_for_origin_files,
        combined_document_symbols, fresh_unchanged_file_uris,
        load_unchanged_document_symbol_extractions, reference_route_summary_for_origin_files,
        workspace_file_hashes,
    },
};

use clap::Parser;
use sha2::Digest;
use std::{
    collections::{BTreeMap, HashSet},
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
        Command::Fts {
            db,
            analysis_workers,
            no_rust,
            no_csharp,
            no_submodules,
        } => {
            let total_timer = Stopwatch::start_new();
            let mut benchmark = BenchmarkSummary::new();
            benchmark.insert_label("execution_mode", "fts_file_content_tantivy_index");
            benchmark.insert_label("mode", "fts");

            let workspace_root = std::env::current_dir()
                .map_err(|source| ExtractError::io("read current directory", None, source))?;
            let fts_config = resolve_cli_fts_config(&config, &workspace_root)?;
            let analysis_worker_count = resolve_cli_fts_analysis_workers(
                &config,
                &workspace_root,
                analysis_workers,
                &fts_config,
            )?;
            validate_single_route_jobs("--analysis-workers", analysis_worker_count)?;
            benchmark.insert_count("analysis_workers", analysis_worker_count);

            let writer_ready_timer = Stopwatch::start_new();
            let db = resolve_cli_fts_database_path(db, &config, &workspace_root, &fts_config)?;
            let index_path = fts_index_path_for_db(&db);
            let store = start_writer(db.clone(), &config, &workspace_root).await?;
            benchmark.insert_duration_ms("writer_ready", writer_ready_timer.elapsed());

            let fts_runner_timer = Stopwatch::start_new();
            let summary = FtsExtractionRunner::run(
                &store,
                &workspace_root,
                &db,
                &index_path,
                &fts_config,
                FtsExtractionOptions::new(no_rust, no_csharp, no_submodules),
                analysis_worker_count,
            )
            .await;
            benchmark.insert_duration_ms("fts_runner", fts_runner_timer.elapsed());

            let writer_shutdown_timer = Stopwatch::start_new();
            shutdown_writer(&store).await?;
            benchmark.insert_duration_ms("writer_shutdown", writer_shutdown_timer.elapsed());

            let mut summary = summary?;
            benchmark.extend_from(&summary.benchmark);
            benchmark.insert_duration_ms("total", total_timer.elapsed());
            summary.benchmark = benchmark;

            print_fts_summary(&summary);
            print_benchmark_summary(&summary.benchmark);
        }
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
            let summary = run_shared_rust_route_batch(
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
        Command::CSharpFile {
            db,
            solution,
            csharp_ls,
            calls,
            references,
            symbols,
            file,
        } => {
            let mode = resolve_csharp_file_mode(calls, references, symbols)?;
            let provider = CSharpLsProvider::new();
            let current_dir = std::env::current_dir()
                .map_err(|source| ExtractError::io("read current directory", None, source))?;
            let plan =
                resolve_csharp_extractor_plan(&config, &current_dir, csharp_ls, solution, None)?;
            let solution_model =
                csharp_ls_lib::load_solution(plan.solution()).map_err(|source| {
                    ExtractError::csharp_ls_lib("load C# solution for csharp-file", source)
                })?;
            let project_match =
                csharp_ls_lib::project_for_file(&solution_model, &file).map_err(|source| {
                    ExtractError::csharp_ls_lib("match C# file to project", source)
                })?;
            let workspace_root = csharp_workspace_root(plan.solution())?;
            let package_path = csharp_package_path(&project_match.project_path)?;
            let document_request =
                validate_document_symbol_batch_request(DocumentSymbolBatchRequest {
                    package_path,
                    workspace_root: workspace_root.clone(),
                    file_paths: vec![project_match.file_path],
                })?;
            let workspace_root_uri = file_uri(plan.solution())?;
            let db = resolve_cli_database_path(db, &config, &workspace_root)?;
            let store = start_writer(db, &config, &workspace_root).await?;

            let extractions =
                extract_csharp_file_with_single_worker(&provider, &plan, document_request, mode)
                    .await?;
            let summary =
                persist_csharp_file_extractions(&store, &workspace_root_uri, mode, &extractions)
                    .await?;
            shutdown_writer(&store).await?;

            print_csharp_file_summary(mode, &summary);
        }
        Command::CSharpFileDeleted {
            db,
            solution,
            csharp_ls,
            file,
        } => {
            let current_dir = std::env::current_dir()
                .map_err(|source| ExtractError::io("read current directory", None, source))?;
            let plan =
                resolve_csharp_extractor_plan(&config, &current_dir, csharp_ls, solution, None)?;
            let workspace_root = csharp_workspace_root(plan.solution())?;
            let deleted_file_path =
                resolve_csharp_deleted_file_path(&workspace_root, &current_dir, &file)?;
            let deleted_file_uri = file_uri(&deleted_file_path)?;
            let relative_path =
                semantic_graph_extract::document_symbols::paths::workspace_relative_path(
                    &workspace_root,
                    &deleted_file_path,
                )?;
            let workspace_root_uri = file_uri(plan.solution())?;
            let db = resolve_cli_database_path(db, &config, &workspace_root)?;
            let store = start_writer(db, &config, &workspace_root).await?;

            let summary = ExtractionPersister
                .mark_deleted_file_stale(
                    &store,
                    &workspace_root_uri,
                    &deleted_file_uri,
                    GraphLanguage::CSharp,
                    ProviderId::csharp_language_server(),
                    "csharp-file-deleted",
                )
                .await?;
            shutdown_writer(&store).await?;

            print_csharp_file_deleted_summary(&relative_path, &summary);
        }
        Command::CSharpProject {
            db,
            solution,
            csharp_ls,
            process_workers,
            calls,
            references,
            symbols,
            project_or_root,
        } => {
            let routes = resolve_csharp_workspace_routes(calls, references, symbols);
            let provider = CSharpLsProvider::new();
            let current_dir = std::env::current_dir()
                .map_err(|source| ExtractError::io("read current directory", None, source))?;
            let plan = resolve_csharp_extractor_plan(
                &config,
                &current_dir,
                csharp_ls,
                solution,
                process_workers,
            )?;
            let solution_model =
                csharp_ls_lib::load_solution(plan.solution()).map_err(|source| {
                    ExtractError::csharp_ls_lib("load C# solution for csharp-project", source)
                })?;
            let discovery_timer = Stopwatch::start_new();
            let file_paths = csharp_ls_lib::project_source_files(&solution_model, &project_or_root)
                .map_err(|source| {
                    ExtractError::csharp_ls_lib("discover C# project source files", source)
                })?;
            let discovery_elapsed = discovery_timer.elapsed();
            let workspace_root = csharp_workspace_root(plan.solution())?;
            let package_path = csharp_project_package_path(&project_or_root)?;
            let document_request =
                validate_document_symbol_batch_request(DocumentSymbolBatchRequest {
                    workspace_root: workspace_root.clone(),
                    package_path,
                    file_paths,
                })?;
            let summary = run_csharp_route_batch(
                &config,
                db,
                &provider,
                &plan,
                document_request,
                routes,
                CSharpRouteBatchContext::new(
                    CSharpRouteBatchScope::Project,
                    Some(discovery_elapsed),
                ),
            )
            .await?;

            print_csharp_route_batch_summary("project", routes, &summary);
            print_benchmark_summary(&summary.benchmark);
        }
        Command::CSharpSolution {
            db,
            solution,
            csharp_ls,
            process_workers,
            calls,
            references,
            symbols,
        } => {
            let routes = resolve_csharp_workspace_routes(calls, references, symbols);
            let provider = CSharpLsProvider::new();
            let current_dir = std::env::current_dir()
                .map_err(|source| ExtractError::io("read current directory", None, source))?;
            let plan = resolve_csharp_extractor_plan(
                &config,
                &current_dir,
                csharp_ls,
                solution,
                process_workers,
            )?;
            let discovery_timer = Stopwatch::start_new();
            let file_paths = provider.discover_csharp_solution_source_files(plan.solution())?;
            let discovery_elapsed = discovery_timer.elapsed();
            let workspace_root = csharp_workspace_root(plan.solution())?;
            let document_request =
                validate_document_symbol_batch_request(DocumentSymbolBatchRequest {
                    package_path: workspace_root.clone(),
                    workspace_root: workspace_root.clone(),
                    file_paths,
                })?;
            let summary = run_csharp_route_batch(
                &config,
                db,
                &provider,
                &plan,
                document_request,
                routes,
                CSharpRouteBatchContext::new(
                    CSharpRouteBatchScope::Solution,
                    Some(discovery_elapsed),
                ),
            )
            .await?;

            print_csharp_route_batch_summary("solution", routes, &summary);
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

async fn extract_csharp_file_with_single_worker(
    provider: &CSharpLsProvider,
    plan: &ResolvedCSharpExtractorPlan,
    document_request: DocumentSymbolBatchRequest,
    mode: CSharpFileMode,
) -> ExtractResult<CSharpFileExtractions> {
    let mut worker = start_csharp_worker(plan).await?;
    let result = extract_csharp_file_with_worker(
        provider,
        &mut worker,
        document_request,
        mode,
        plan.solution(),
    )
    .await;
    let shutdown_result = worker.shutdown().await.map_err(|source| {
        ExtractError::csharp_ls_lib("shutdown single csharp-file worker", source)
    });

    match (result, shutdown_result) {
        (Ok(extractions), Ok(())) => Ok(extractions),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
    }
}

async fn extract_csharp_file_with_worker(
    provider: &CSharpLsProvider,
    worker: &mut csharp_ls_lib::CSharpLsWorker,
    document_request: DocumentSymbolBatchRequest,
    mode: CSharpFileMode,
    solution_path: &Path,
) -> ExtractResult<CSharpFileExtractions> {
    let file_scope_key = file_uri(&document_request.file_paths[0])?;
    let document_symbols =
        csharp_document_symbols_with_worker(provider, worker, document_request.clone()).await?;

    if !mode.includes_references() && !mode.includes_calls() {
        return Ok(CSharpFileExtractions {
            file_scope_key,
            document_symbols,
            references: None,
            calls: None,
        });
    }

    let (relation_document_request, relation_document_symbols) =
        csharp_file_relation_document_symbols(provider, worker, &document_request, solution_path)
            .await?;

    let references =
        if mode.includes_references() {
            let reference_targets = provider.reference_targets_for_document_symbols(
                &relation_document_request,
                &relation_document_symbols,
            )?;
            let mut reference_sets = Vec::with_capacity(reference_targets.len());
            for target in &reference_targets {
                reference_sets.push(worker.references_for_symbol(target).await.map_err(
                    |source| {
                        ExtractError::csharp_ls_lib("csharp-ls-lib references_for_symbol", source)
                    },
                )?);
            }
            let reference_sets = csharp_reference_sets_for_file_relations(
                reference_sets,
                &document_request.file_paths[0],
            );
            Some(provider.map_reference_sets(
                &relation_document_request,
                relation_document_symbols.clone(),
                reference_sets,
                reference_targets.len(),
            )?)
        } else {
            None
        };

    let calls = if mode.includes_calls() {
        let call_targets = provider.call_targets_for_document_symbols(
            &relation_document_request,
            &relation_document_symbols,
        )?;
        let mut incoming_call_sets = Vec::with_capacity(call_targets.len());
        for target in &call_targets {
            incoming_call_sets.push(worker.incoming_calls_for_symbol(target).await.map_err(
                |source| {
                    ExtractError::csharp_ls_lib("csharp-ls-lib incoming_calls_for_symbol", source)
                },
            )?);
        }
        let calls = provider.map_incoming_call_sets(
            &relation_document_request,
            relation_document_symbols,
            incoming_call_sets,
            call_targets.len(),
        )?;
        Some(call_extraction_for_file_relations(calls, &file_scope_key))
    } else {
        None
    };

    Ok(CSharpFileExtractions {
        file_scope_key,
        document_symbols,
        references,
        calls,
    })
}

async fn csharp_document_symbols_with_worker(
    provider: &CSharpLsProvider,
    worker: &mut csharp_ls_lib::CSharpLsWorker,
    document_request: DocumentSymbolBatchRequest,
) -> ExtractResult<DocumentSymbolBatchExtraction> {
    let document_symbol_items = worker
        .document_symbols_for_files(document_request.file_paths.clone())
        .await
        .map_err(|source| {
            ExtractError::csharp_ls_lib("csharp-ls-lib document_symbols_for_files", source)
        })?;

    provider.map_document_symbol_items(document_request, document_symbol_items)
}

async fn csharp_file_relation_document_symbols(
    provider: &CSharpLsProvider,
    worker: &mut csharp_ls_lib::CSharpLsWorker,
    document_request: &DocumentSymbolBatchRequest,
    solution_path: &Path,
) -> ExtractResult<(DocumentSymbolBatchRequest, DocumentSymbolBatchExtraction)> {
    let mut file_paths = provider.discover_csharp_solution_source_files(solution_path)?;
    file_paths.push(document_request.file_paths[0].clone());

    let relation_document_request =
        validate_document_symbol_batch_request(DocumentSymbolBatchRequest {
            package_path: document_request.workspace_root.clone(),
            workspace_root: document_request.workspace_root.clone(),
            file_paths,
        })?;
    let relation_document_symbols =
        csharp_document_symbols_with_worker(provider, worker, relation_document_request.clone())
            .await?;

    Ok((relation_document_request, relation_document_symbols))
}

fn csharp_reference_sets_for_file_relations(
    mut reference_sets: Vec<csharp_ls_lib::ResolvedReferenceSet>,
    file_path: &Path,
) -> Vec<csharp_ls_lib::ResolvedReferenceSet> {
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

async fn persist_csharp_file_extractions(
    store: &WriteHandle,
    workspace_root_uri: &str,
    mode: CSharpFileMode,
    extractions: &CSharpFileExtractions,
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
            "csharp-language-server",
            "csharp-file",
            "csharp-file extraction produced no persistence work",
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

fn print_csharp_file_summary(mode: CSharpFileMode, summary: &PersistenceSummary) {
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

fn print_csharp_file_deleted_summary(relative_path: &str, summary: &PersistenceSummary) {
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

fn print_fts_summary(summary: &FtsExtractionSummary) {
    print_fts_summary_with_mode("fts", summary);
}

fn print_fts_summary_with_mode(mode: &str, summary: &FtsExtractionSummary) {
    println!(
        "mode={} workspace={} run={} scanned_files={} files_hashed={} files_hash_unchanged={} files_changed={} indexed_files={} skipped_files={} skipped_directories={} skipped_by_config={} skipped_by_no_rust={} skipped_by_no_csharp={} skipped_by_no_submodules={} skipped_binary_or_unreadable={} stale_fts_documents_closed={}",
        mode,
        summary.workspace_id,
        summary.run_id,
        summary.scanned_files,
        summary.files_hashed,
        summary.files_hash_unchanged,
        summary.files_changed,
        summary.indexed_files,
        summary.skipped_files,
        summary.skipped_directories,
        summary.skipped_by_config,
        summary.skipped_by_no_rust,
        summary.skipped_by_no_csharp,
        summary.skipped_by_no_submodules,
        summary.skipped_binary_or_unreadable,
        summary.stale_fts_documents_closed
    );
}

fn print_csharp_route_batch_summary(
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

async fn run_shared_rust_route_batch(
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
    benchmark.insert_label("execution_mode", "shared_analysis_snapshot");
    benchmark.insert_label("mode", "shared-workspace");
    benchmark.insert_label("routes", routes.label());
    benchmark.insert_count("files_discovered", document_request.file_paths.len());

    let writer_ready_timer = Stopwatch::start_new();
    let db = resolve_cli_database_path(db, config, &document_request.workspace_root)?;
    let close_stale_document_symbols = db.exists();
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
    benchmark.insert_count("analysis_workers", analysis_worker_count);
    benchmark.insert_count("reference_jobs", reference_jobs);
    benchmark.insert_count("call_jobs", call_jobs);

    let shared_runner_timer = Stopwatch::start_new();
    let summary = SharedWorkspaceExtractionRunner::run(
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
        close_stale_document_symbols,
    )
    .await;
    benchmark.insert_duration_ms("shared_workspace_runner", shared_runner_timer.elapsed());

    let writer_shutdown_timer = Stopwatch::start_new();
    shutdown_writer(&store).await?;
    benchmark.insert_duration_ms("writer_shutdown", writer_shutdown_timer.elapsed());
    let mut summary = summary?;
    benchmark.extend_from(&summary.benchmark);
    benchmark.insert_duration_ms("total", total_timer.elapsed());
    summary.benchmark = benchmark;

    Ok(summary)
}

async fn run_csharp_route_batch(
    config: &Option<PathBuf>,
    db: Option<PathBuf>,
    provider: &CSharpLsProvider,
    plan: &ResolvedCSharpExtractorPlan,
    document_request: DocumentSymbolBatchRequest,
    routes: WorkspaceExtractionRoutes,
    context: CSharpRouteBatchContext,
) -> ExtractResult<WorkspaceExtractionSummary> {
    let total_timer = Stopwatch::start_new();
    let mut benchmark = BenchmarkSummary::new();
    let scope = context.scope();
    let route_prefix = scope.benchmark_prefix();
    if let Some(elapsed) = context.discovery_elapsed() {
        benchmark.insert_duration_ms("discovery", elapsed);
    }
    benchmark.insert_label("execution_mode", "csharp_process_pool");
    benchmark.insert_label("mode", format!("csharp-{}", scope.label()));
    benchmark.insert_label("routes", routes.label());
    benchmark.insert_count("process_workers", plan.process_workers());
    benchmark.insert_count(
        "reference_jobs",
        if routes.includes_references() {
            plan.process_workers()
        } else {
            0
        },
    );
    benchmark.insert_count(
        "call_jobs",
        if routes.includes_calls() {
            plan.process_workers()
        } else {
            0
        },
    );
    benchmark.insert_count("files_discovered", document_request.file_paths.len());
    benchmark.insert_label(
        &format!("{route_prefix}.execution_mode"),
        "csharp_process_pool",
    );
    benchmark.insert_label(&format!("{route_prefix}.routes"), routes.label());
    benchmark.insert_count(
        &format!("{route_prefix}.process_workers"),
        plan.process_workers(),
    );
    benchmark.insert_count(
        &format!("{route_prefix}.input_files"),
        document_request.file_paths.len(),
    );

    let workspace_root_uri_timer = Stopwatch::start_new();
    let workspace_root_uri = file_uri(plan.solution())?;
    benchmark.insert_duration_ms(
        &format!("{route_prefix}.workspace_root_uri"),
        workspace_root_uri_timer.elapsed(),
    );
    let writer_ready_timer = Stopwatch::start_new();
    let db = resolve_cli_database_path(db, config, &document_request.workspace_root)?;
    let store = start_writer(db, config, &document_request.workspace_root).await?;
    benchmark.insert_duration_ms("writer_ready", writer_ready_timer.elapsed());

    let incremental_solution = scope == CSharpRouteBatchScope::Solution;
    let file_hash_timer = Stopwatch::start_new();
    let file_hashes = if incremental_solution {
        workspace_file_hashes(&document_request)?
    } else {
        Vec::new()
    };
    benchmark.insert_duration_ms(
        &format!("{route_prefix}.file_hashes"),
        file_hash_timer.elapsed(),
    );
    benchmark.insert_count(&format!("{route_prefix}.files_hashed"), file_hashes.len());

    let workspace_id_timer = Stopwatch::start_new();
    let existing_workspace_id_value = if incremental_solution {
        store
            .workspace_id(&workspace_root_uri)
            .await
            .map_err(ExtractError::storage)?
    } else {
        None
    };
    benchmark.insert_duration_ms(
        &format!("{route_prefix}.existing_workspace_id"),
        workspace_id_timer.elapsed(),
    );

    let unchanged_file_uri_timer = Stopwatch::start_new();
    let fresh_unchanged_file_uris = if let Some(workspace_id) = existing_workspace_id_value {
        fresh_unchanged_file_uris(
            &store,
            workspace_id,
            RouteName::CSHARP_DOCUMENT_SYMBOLS.as_str(),
            provider.provider_id(),
            &file_hashes,
        )
        .await?
    } else {
        HashSet::new()
    };
    benchmark.insert_duration_ms(
        &format!("{route_prefix}.unchanged_file_hash_lookup"),
        unchanged_file_uri_timer.elapsed(),
    );

    let loaded_symbols_timer = Stopwatch::start_new();
    let loaded_document_symbol_extractions = if let Some(workspace_id) = existing_workspace_id_value
    {
        load_unchanged_document_symbol_extractions(
            &store,
            workspace_id,
            provider.provider_id(),
            csharp_ls_lib::provider_version(),
            &fresh_unchanged_file_uris,
        )
        .await?
    } else {
        Vec::new()
    };
    let loaded_file_uris = loaded_document_symbol_extractions
        .iter()
        .map(|extraction| extraction.source_file.uri.clone())
        .collect::<HashSet<_>>();
    benchmark.insert_duration_ms(
        &format!("{route_prefix}.unchanged_symbols_load"),
        loaded_symbols_timer.elapsed(),
    );

    let changed_file_hashes = if incremental_solution {
        file_hashes
            .iter()
            .filter(|file_hash| !loaded_file_uris.contains(&file_hash.uri))
            .cloned()
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let changed_file_paths = if incremental_solution {
        changed_file_hashes
            .iter()
            .map(|file_hash| file_hash.file_path.clone())
            .collect::<Vec<_>>()
    } else {
        document_request.file_paths.clone()
    };
    let changed_file_uris = changed_file_hashes
        .iter()
        .map(|file_hash| file_hash.uri.clone())
        .collect::<Vec<_>>();
    let changed_file_uri_set = changed_file_uris.iter().cloned().collect::<HashSet<_>>();
    let use_origin_file_relation_batches = incremental_solution && !loaded_file_uris.is_empty();
    benchmark.insert_count(
        &format!("{route_prefix}.files_hash_unchanged"),
        loaded_file_uris.len(),
    );
    benchmark.insert_count(
        &format!("{route_prefix}.files_changed"),
        changed_file_paths.len(),
    );

    let process_pool_start_timer = Stopwatch::start_new();
    let mut worker_pool = if incremental_solution && changed_file_paths.is_empty() {
        None
    } else {
        Some(
            csharp_ls_lib::CSharpLsWorkerPool::start(
                plan.binary().clone(),
                plan.solution().clone(),
                plan.log_level().to_string(),
                plan.features().to_vec(),
                plan.startup_timeout_ms(),
                plan.request_timeout_ms(),
                plan.process_workers(),
            )
            .await
            .map_err(|source| ExtractError::csharp_ls_lib("start C# worker pool", source))?,
        )
    };
    benchmark.insert_duration_ms(
        &format!("{route_prefix}.process_pool_start"),
        process_pool_start_timer.elapsed(),
    );
    let actual_process_workers = worker_pool
        .as_ref()
        .map(csharp_ls_lib::CSharpLsWorkerPool::worker_count)
        .unwrap_or(0);
    benchmark.insert_count("actual_process_workers", actual_process_workers);
    benchmark.insert_count(
        &format!("{route_prefix}.actual_process_workers"),
        actual_process_workers,
    );

    let mut changed_document_request = document_request.clone();
    changed_document_request.file_paths = changed_file_paths.clone();
    let document_symbols_query_timer = Stopwatch::start_new();
    let document_symbol_items = if let Some(worker_pool) = worker_pool.as_mut() {
        worker_pool
            .document_symbols_for_files(changed_file_paths.clone())
            .await
            .map_err(|source| {
                ExtractError::csharp_ls_lib("csharp-ls-lib document_symbols_for_files", source)
            })?
    } else {
        Vec::new()
    };
    benchmark.insert_duration_ms(
        &format!("{route_prefix}.document_symbols_query"),
        document_symbols_query_timer.elapsed(),
    );
    let document_symbols_map_timer = Stopwatch::start_new();
    let changed_document_symbols =
        provider.map_document_symbol_items(changed_document_request, document_symbol_items)?;
    let document_symbols = if incremental_solution {
        combined_document_symbols(
            provider.provider_id(),
            "csharp-ls-lib",
            changed_document_symbols.clone(),
            loaded_document_symbol_extractions,
        )
    } else {
        changed_document_symbols.clone()
    };
    benchmark.insert_duration_ms(
        &format!("{route_prefix}.document_symbols_map"),
        document_symbols_map_timer.elapsed(),
    );
    benchmark.insert_count("document_files", document_symbols.extractions.len());
    benchmark.insert_count(
        &format!("{route_prefix}.document_files"),
        document_symbols.extractions.len(),
    );
    benchmark.insert_count(
        &format!("{route_prefix}.document_symbols"),
        document_symbol_count(&document_symbols),
    );
    benchmark.insert_count(
        &format!("{route_prefix}.document_files_extracted"),
        changed_document_symbols.extractions.len(),
    );
    benchmark.insert_count(
        &format!("{route_prefix}.document_files_loaded"),
        document_symbols.extractions.len() - changed_document_symbols.extractions.len(),
    );

    let mut document_summary = if routes.includes_symbols() {
        let document_symbols_persist_timer = Stopwatch::start_new();
        let summary = if incremental_solution && changed_document_symbols.extractions.is_empty() {
            let workspace_id = existing_workspace_id_value.ok_or_else(|| {
                ExtractError::response_shape(
                    provider.provider_id().as_str(),
                    "csharp-solution --symbols",
                    format!("workspace {workspace_root_uri} is missing"),
                )
            })?;
            empty_persistence_summary(workspace_id, 0)
        } else {
            ExtractionPersister
                .persist_document_symbol_batch(
                    &store,
                    &workspace_root_uri,
                    &changed_document_symbols,
                )
                .await?
        };
        benchmark.insert_duration_ms(
            &format!("{route_prefix}.document_symbols_persist"),
            document_symbols_persist_timer.elapsed(),
        );
        benchmark.insert_label(
            &format!("{route_prefix}.document_symbols_write_mode"),
            "document_symbol_batch",
        );
        summary
    } else {
        empty_persistence_summary(
            existing_csharp_workspace_id(&store, &workspace_root_uri).await?,
            0,
        )
    };

    let workspace_fingerprint = csharp_workspace_fingerprint(&document_symbols);
    let mut reference_summary = empty_persistence_summary(document_summary.workspace_id, 0);
    let mut call_summary = empty_persistence_summary(document_summary.workspace_id, 0);
    let mut reference_route_summary = empty_reference_route_summary();
    let mut call_route_summary = empty_call_route_summary();

    if routes.includes_references() && (!incremental_solution || !changed_file_uris.is_empty()) {
        let reference_targets_timer = Stopwatch::start_new();
        let reference_targets = provider
            .reference_targets_for_document_symbols(&document_request, &document_symbols)?;
        benchmark.insert_count(
            &format!("{route_prefix}.reference_targets"),
            reference_targets.len(),
        );
        benchmark.insert_duration_ms(
            &format!("{route_prefix}.reference_targets_build"),
            reference_targets_timer.elapsed(),
        );
        let reference_work_build_timer = Stopwatch::start_new();
        let reference_targets_queried = reference_targets.len();
        let work_items = csharp_reference_work_items_by_file(reference_targets);
        benchmark.insert_count(
            &format!("{route_prefix}.reference_work_items"),
            work_items.len(),
        );
        benchmark.insert_duration_ms(
            &format!("{route_prefix}.reference_work_build"),
            reference_work_build_timer.elapsed(),
        );
        let reference_work_timer = Stopwatch::start_new();
        let file_results = if work_items.is_empty() {
            Vec::new()
        } else {
            worker_pool
                .as_mut()
                .ok_or_else(|| {
                    ExtractError::response_shape(
                        provider.provider_id().as_str(),
                        "csharp-solution",
                        "reference work was scheduled without a C# worker pool",
                    )
                })?
                .file_semantic_work_items(work_items)
                .await
                .map_err(|source| ExtractError::csharp_ls_lib("C# reference work items", source))?
        };
        benchmark.insert_duration_ms(
            &format!("{route_prefix}.reference_work"),
            reference_work_timer.elapsed(),
        );
        let reference_sets = file_results
            .into_iter()
            .flat_map(|result| result.reference_sets)
            .collect::<Vec<_>>();
        let references_map_timer = Stopwatch::start_new();
        let mut extraction = provider.map_reference_sets(
            &document_request,
            document_symbols.clone(),
            reference_sets,
            reference_targets_queried,
        )?;
        benchmark.insert_duration_ms(
            &format!("{route_prefix}.references_map"),
            references_map_timer.elapsed(),
        );
        extraction.workspace_fingerprint = workspace_fingerprint.clone();
        reference_route_summary = if use_origin_file_relation_batches {
            reference_route_summary_for_origin_files(&extraction, &changed_file_uri_set)
        } else {
            extraction.summary.clone()
        };
        let references_persist_timer = Stopwatch::start_new();
        reference_summary = if use_origin_file_relation_batches {
            ExtractionPersister
                .persist_reference_origin_file_batches_with_route_write_batch(
                    &store,
                    &workspace_root_uri,
                    &extraction,
                    &changed_file_uris,
                )
                .await?
        } else {
            ExtractionPersister
                .persist_reference_batch(&store, &workspace_root_uri, &extraction)
                .await?
        };
        benchmark.insert_duration_ms(
            &format!("{route_prefix}.references_persist"),
            references_persist_timer.elapsed(),
        );
        benchmark.insert_label(
            &format!("{route_prefix}.references_write_mode"),
            if use_origin_file_relation_batches {
                "route_write_batch_origin_file"
            } else {
                "reference_batch"
            },
        );
        if document_summary.workspace_id == 0 {
            document_summary.workspace_id = reference_summary.workspace_id;
        }
    }

    if routes.includes_calls() && (!incremental_solution || !changed_file_uris.is_empty()) {
        let call_targets_timer = Stopwatch::start_new();
        let call_targets =
            provider.call_targets_for_document_symbols(&document_request, &document_symbols)?;
        benchmark.insert_count(&format!("{route_prefix}.call_targets"), call_targets.len());
        benchmark.insert_duration_ms(
            &format!("{route_prefix}.call_targets_build"),
            call_targets_timer.elapsed(),
        );
        let call_work_build_timer = Stopwatch::start_new();
        let call_targets_queried = call_targets.len();
        let work_items = csharp_call_work_items_by_file(call_targets);
        benchmark.insert_count(&format!("{route_prefix}.call_work_items"), work_items.len());
        benchmark.insert_duration_ms(
            &format!("{route_prefix}.call_work_build"),
            call_work_build_timer.elapsed(),
        );
        let call_work_timer = Stopwatch::start_new();
        let file_results = if work_items.is_empty() {
            Vec::new()
        } else {
            worker_pool
                .as_mut()
                .ok_or_else(|| {
                    ExtractError::response_shape(
                        provider.provider_id().as_str(),
                        "csharp-solution",
                        "call work was scheduled without a C# worker pool",
                    )
                })?
                .file_semantic_work_items(work_items)
                .await
                .map_err(|source| ExtractError::csharp_ls_lib("C# call work items", source))?
        };
        benchmark.insert_duration_ms(
            &format!("{route_prefix}.call_work"),
            call_work_timer.elapsed(),
        );
        let incoming_call_sets = file_results
            .into_iter()
            .flat_map(|result| result.incoming_call_sets)
            .collect::<Vec<_>>();
        let calls_map_timer = Stopwatch::start_new();
        let mut extraction = provider.map_incoming_call_sets(
            &document_request,
            document_symbols,
            incoming_call_sets,
            call_targets_queried,
        )?;
        benchmark.insert_duration_ms(
            &format!("{route_prefix}.calls_map"),
            calls_map_timer.elapsed(),
        );
        extraction.workspace_fingerprint = workspace_fingerprint;
        call_route_summary = if use_origin_file_relation_batches {
            call_route_summary_for_origin_files(&extraction, &changed_file_uri_set)
        } else {
            extraction.summary.clone()
        };
        let calls_persist_timer = Stopwatch::start_new();
        call_summary = if use_origin_file_relation_batches {
            ExtractionPersister
                .persist_call_origin_file_batches_with_route_write_batch(
                    &store,
                    &workspace_root_uri,
                    &extraction,
                    &changed_file_uris,
                )
                .await?
        } else {
            ExtractionPersister
                .persist_call_batch(&store, &workspace_root_uri, &extraction)
                .await?
        };
        benchmark.insert_duration_ms(
            &format!("{route_prefix}.calls_persist"),
            calls_persist_timer.elapsed(),
        );
        benchmark.insert_label(
            &format!("{route_prefix}.calls_write_mode"),
            if use_origin_file_relation_batches {
                "route_write_batch_origin_file"
            } else {
                "call_batch"
            },
        );
        if document_summary.workspace_id == 0 {
            document_summary.workspace_id = call_summary.workspace_id;
        }
    }

    let process_pool_shutdown_timer = Stopwatch::start_new();
    if let Some(worker_pool) = worker_pool {
        worker_pool
            .shutdown()
            .await
            .map_err(|source| ExtractError::csharp_ls_lib("shutdown C# worker pool", source))?;
    }
    benchmark.insert_duration_ms(
        &format!("{route_prefix}.process_pool_shutdown"),
        process_pool_shutdown_timer.elapsed(),
    );
    let writer_shutdown_timer = Stopwatch::start_new();
    shutdown_writer(&store).await?;
    benchmark.insert_duration_ms("writer_shutdown", writer_shutdown_timer.elapsed());

    benchmark.insert_duration_ms(&format!("{route_prefix}.total"), total_timer.elapsed());
    benchmark.insert_duration_ms("total", total_timer.elapsed());
    Ok(WorkspaceExtractionSummary {
        benchmark,
        document_summary,
        reference_summary,
        call_summary,
        reference_route_summary,
        call_route_summary,
    })
}

fn csharp_reference_work_items_by_file(
    targets: Vec<csharp_ls_lib::ResolvedReferenceTarget>,
) -> Vec<csharp_ls_lib::FileSemanticWork> {
    let mut targets_by_file =
        BTreeMap::<PathBuf, Vec<csharp_ls_lib::ResolvedReferenceTarget>>::new();
    for target in targets {
        targets_by_file
            .entry(target.file_path.clone())
            .or_default()
            .push(target);
    }

    targets_by_file
        .into_iter()
        .map(
            |(file_path, reference_targets)| csharp_ls_lib::FileSemanticWork {
                file_path,
                reference_targets,
                call_targets: Vec::new(),
            },
        )
        .collect()
}

fn csharp_call_work_items_by_file(
    targets: Vec<csharp_ls_lib::ResolvedCallTarget>,
) -> Vec<csharp_ls_lib::FileSemanticWork> {
    let mut targets_by_file = BTreeMap::<PathBuf, Vec<csharp_ls_lib::ResolvedCallTarget>>::new();
    for target in targets {
        targets_by_file
            .entry(target.file_path.clone())
            .or_default()
            .push(target);
    }

    targets_by_file
        .into_iter()
        .map(
            |(file_path, call_targets)| csharp_ls_lib::FileSemanticWork {
                file_path,
                reference_targets: Vec::new(),
                call_targets,
            },
        )
        .collect()
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

async fn start_csharp_worker(
    plan: &ResolvedCSharpExtractorPlan,
) -> ExtractResult<csharp_ls_lib::CSharpLsWorker> {
    csharp_ls_lib::CSharpLsWorker::start(
        plan.binary().clone(),
        plan.solution().clone(),
        plan.log_level().to_string(),
        plan.features().to_vec(),
        plan.startup_timeout_ms(),
        plan.request_timeout_ms(),
    )
    .await
    .map_err(|source| ExtractError::csharp_ls_lib("start C# worker", source))
}

fn csharp_workspace_root(solution_path: &Path) -> ExtractResult<PathBuf> {
    solution_path
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            ExtractError::invalid_path(
                solution_path,
                PathBuf::new(),
                "C# solution path has no parent directory",
            )
        })
}

fn resolve_csharp_deleted_file_path(
    workspace_root: &Path,
    current_dir: &Path,
    file_path: &Path,
) -> ExtractResult<PathBuf> {
    let current_dir_candidate = if file_path.is_absolute() {
        file_path.to_path_buf()
    } else {
        current_dir.join(file_path)
    };
    let current_dir_candidate = normalize_lexical_path(&current_dir_candidate);
    let resolved = if current_dir_candidate.starts_with(workspace_root) {
        current_dir_candidate
    } else if file_path.is_absolute() {
        return Err(ExtractError::invalid_path(
            current_dir_candidate,
            workspace_root,
            "deleted csharp-file path is outside the resolved solution directory",
        ));
    } else {
        normalize_lexical_path(&workspace_root.join(file_path))
    };

    if !resolved.starts_with(workspace_root) {
        return Err(ExtractError::invalid_path(
            resolved,
            workspace_root,
            "deleted csharp-file path is outside the resolved solution directory",
        ));
    }
    if resolved.exists() && !resolved.is_file() {
        return Err(ExtractError::invalid_path(
            resolved,
            workspace_root,
            "deleted csharp-file path must be a file path",
        ));
    }

    Ok(resolved)
}

fn csharp_package_path(project_path: &Path) -> ExtractResult<PathBuf> {
    project_path.parent().map(Path::to_path_buf).ok_or_else(|| {
        ExtractError::invalid_path(
            project_path,
            PathBuf::new(),
            "C# project path has no parent directory",
        )
    })
}

fn csharp_project_package_path(project_or_root: &Path) -> ExtractResult<PathBuf> {
    let canonical = project_or_root
        .canonicalize()
        .map_err(|source| ExtractError::io("canonicalize C# project boundary", None, source))?;
    if canonical
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("csproj"))
    {
        csharp_package_path(&canonical)
    } else {
        Ok(canonical)
    }
}

async fn existing_csharp_workspace_id(
    store: &WriteHandle,
    workspace_root_uri: &str,
) -> ExtractResult<i64> {
    store
        .workspace_id(workspace_root_uri)
        .await
        .map_err(ExtractError::storage)?
        .ok_or_else(|| {
            ExtractError::response_shape(
                "csharp-language-server",
                "csharp route-only extraction",
                format!(
                    "workspace {workspace_root_uri} is missing; run csharp-solution --symbols, csharp-project --symbols, or csharp-file --symbols first"
                ),
            )
        })
}

fn empty_persistence_summary(workspace_id: i64, run_id: i64) -> PersistenceSummary {
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

fn csharp_workspace_fingerprint(extraction: &DocumentSymbolBatchExtraction) -> String {
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

    let mut hasher = sha2::Sha256::new();
    for entry in entries {
        hasher.update(entry.as_bytes());
        hasher.update(b"\n");
    }

    hex::encode(hasher.finalize())
}

fn document_symbol_count(extraction: &DocumentSymbolBatchExtraction) -> usize {
    extraction
        .extractions
        .iter()
        .map(|extraction| extraction.symbols.len())
        .sum()
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

fn resolve_cli_fts_config(
    config: &Option<PathBuf>,
    workspace_root: &Path,
) -> ExtractResult<semantic_graph_config::FtsConfig> {
    let config_path = match config {
        Some(path) => Some(path.clone()),
        None => discover_config(workspace_root).map_err(ExtractError::config)?,
    };

    let Some(config_path) = config_path else {
        return Ok(semantic_graph_config::FtsConfig::default());
    };

    let config = load_config(config_path).map_err(ExtractError::config)?;
    Ok(config.fts().clone())
}

fn fts_index_path_for_db(db: &Path) -> PathBuf {
    db.with_extension("tantivy")
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
