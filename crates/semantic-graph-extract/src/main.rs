use semantic_graph_extract::{
    CliExtractorPlanOptions, ExtractError, ExtractResult, ResolvedExtractorPlan,
    benchmark::{BenchmarkSummary, Stopwatch},
    document_symbols::paths::{
        file_uri, validate_document_symbol_batch_request, validate_document_symbol_request,
        workspace_relative_path,
    },
    model::{
        CallBatchExtraction, CallBatchRequest, DocumentSymbolBatchExtraction,
        DocumentSymbolBatchRequest, DocumentSymbolRequest, ReferenceBatchExtraction,
        ReferenceBatchRequest,
    },
    persist::{ExtractionPersister, PersistenceSummary},
    provider::DocumentSymbolProvider,
    providers::rust_analyzer::RustAnalyzerProvider,
    workspace_all::{ThreadedWorkspaceAllConfig, ThreadedWorkspaceAllRunner},
};

use semantic_graph_config::{
    ExtractorMode, LoadOptions, discover_config, load_config, resolve_database_path,
};
use semantic_graph_db_manager::{Config, WriteHandle, WriteManager};

use clap::{Parser, Subcommand};
use std::{
    error::Error,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Parser)]
#[command(about = "Language-server-backed semantic graph extraction prototype")]
struct Cli {
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(name = "rust-file")]
    RustFile {
        #[arg(long)]
        db: Option<PathBuf>,
        #[arg(long, value_name = "WORKSPACE_ROOT", default_value = ".")]
        workspace_root: PathBuf,
        #[arg(long)]
        calls: bool,
        #[arg(long)]
        references: bool,
        #[arg(long)]
        symbols: bool,
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },
    #[command(name = "rust-file-deleted")]
    RustFileDeleted {
        #[arg(long)]
        db: Option<PathBuf>,
        #[arg(long, value_name = "WORKSPACE_ROOT", default_value = ".")]
        workspace_root: PathBuf,
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },
    #[command(name = "rust-document-symbols")]
    SingleFile {
        #[arg(long)]
        db: Option<PathBuf>,
        #[arg(long)]
        workspace_root: PathBuf,
        #[arg(long)]
        package_path: PathBuf,
        #[arg(long)]
        file: PathBuf,
    },
    #[command(name = "rust-crate-document-symbols")]
    CrateBatch {
        #[arg(long)]
        db: Option<PathBuf>,
        #[arg(long)]
        workspace_root: PathBuf,
        #[arg(long)]
        package_path: PathBuf,
    },
    #[command(name = "rust-workspace-document-symbols")]
    WorkspaceBatch {
        #[arg(long)]
        db: Option<PathBuf>,
        #[arg(long)]
        workspace_root: PathBuf,
    },
    #[command(name = "rust-workspace-references")]
    WorkspaceReferences {
        #[arg(long)]
        db: Option<PathBuf>,
        #[arg(long)]
        workspace_root: PathBuf,
    },
    #[command(name = "rust-workspace-calls")]
    WorkspaceCalls {
        #[arg(long)]
        db: Option<PathBuf>,
        #[arg(long)]
        workspace_root: PathBuf,
    },
    #[command(name = "rust-workspace-all")]
    WorkspaceAll {
        #[arg(long)]
        db: Option<PathBuf>,
        #[arg(long)]
        workspace_root: PathBuf,
        #[arg(long)]
        jobs: Option<usize>,
        #[arg(long)]
        reference_jobs: Option<usize>,
        #[arg(long)]
        call_jobs: Option<usize>,
        #[arg(long)]
        analysis_workers: Option<usize>,
        #[arg(long)]
        reference_analysis_workers: Option<usize>,
        #[arg(long)]
        call_analysis_workers: Option<usize>,
        #[arg(long)]
        serial: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RustFileMode {
    Full,
    Symbols,
    References,
    Calls,
}

impl RustFileMode {
    fn includes_symbols(self) -> bool {
        matches!(self, Self::Full | Self::Symbols)
    }

    fn includes_references(self) -> bool {
        matches!(self, Self::Full | Self::References)
    }

    fn includes_calls(self) -> bool {
        matches!(self, Self::Full | Self::Calls)
    }

    fn label(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Symbols => "symbols",
            Self::References => "references",
            Self::Calls => "calls",
        }
    }
}

struct RustFileExtractions {
    file_scope_key: String,
    document_symbols: DocumentSymbolBatchExtraction,
    references: Option<ReferenceBatchExtraction>,
    calls: Option<CallBatchExtraction>,
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        print_error(&error);
        std::process::exit(1);
    }
}

fn print_error(error: &dyn Error) {
    eprintln!("{error}");

    let mut source = error.source();
    while let Some(error) = source {
        eprintln!("caused by: {error}");
        source = error.source();
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
        Command::SingleFile {
            db,
            workspace_root,
            package_path,
            file,
        } => {
            let request = validate_document_symbol_request(DocumentSymbolRequest {
                workspace_root,
                package_path,
                file_path: file,
            })?;
            let workspace_root_uri = file_uri(&request.workspace_root)?;
            let db = resolve_cli_database_path(db, &config, &request.workspace_root)?;
            let store = start_writer(db, &config, &request.workspace_root).await?;

            let provider = RustAnalyzerProvider::new();
            let extraction = provider.extract_document_symbols(request).await?;
            let summary = ExtractionPersister
                .persist_document_symbols(&store, &workspace_root_uri, &extraction)
                .await?;
            shutdown_writer(&store).await?;

            println!(
                "workspace={} run={} files={} nodes={} edges={} occurrences={} evidence={}",
                summary.workspace_id,
                summary.run_id,
                summary.files,
                summary.nodes,
                summary.edges,
                summary.occurrences,
                summary.evidence
            );
        }
        Command::CrateBatch {
            db,
            workspace_root,
            package_path,
        } => {
            let provider = RustAnalyzerProvider::new();
            let file_paths = provider.discover_rust_source_files(&workspace_root, &package_path)?;
            let request = validate_document_symbol_batch_request(DocumentSymbolBatchRequest {
                workspace_root,
                package_path,
                file_paths,
            })?;
            let workspace_root_uri = file_uri(&request.workspace_root)?;
            let db = resolve_cli_database_path(db, &config, &request.workspace_root)?;
            let store = start_writer(db, &config, &request.workspace_root).await?;

            let extraction = provider.extract_document_symbol_batch(request).await?;
            let summary = ExtractionPersister
                .persist_document_symbol_batch(&store, &workspace_root_uri, &extraction)
                .await?;
            shutdown_writer(&store).await?;

            println!(
                "workspace={} run={} files={} nodes={} edges={} occurrences={} evidence={}",
                summary.workspace_id,
                summary.run_id,
                summary.files,
                summary.nodes,
                summary.edges,
                summary.occurrences,
                summary.evidence
            );
        }
        Command::WorkspaceBatch { db, workspace_root } => {
            let provider = RustAnalyzerProvider::new();
            let file_paths = provider.discover_rust_workspace_source_files(&workspace_root)?;
            let request = validate_document_symbol_batch_request(DocumentSymbolBatchRequest {
                package_path: workspace_root.clone(),
                workspace_root,
                file_paths,
            })?;
            let workspace_root_uri = file_uri(&request.workspace_root)?;
            let db = resolve_cli_database_path(db, &config, &request.workspace_root)?;
            let store = start_writer(db, &config, &request.workspace_root).await?;

            let extraction = provider.extract_document_symbol_batch(request).await?;
            let summary = ExtractionPersister
                .persist_document_symbol_batch(&store, &workspace_root_uri, &extraction)
                .await?;
            shutdown_writer(&store).await?;

            println!(
                "workspace={} run={} files={} nodes={} edges={} occurrences={} evidence={}",
                summary.workspace_id,
                summary.run_id,
                summary.files,
                summary.nodes,
                summary.edges,
                summary.occurrences,
                summary.evidence
            );
        }
        Command::WorkspaceReferences { db, workspace_root } => {
            let provider = RustAnalyzerProvider::new();
            let file_paths = provider.discover_rust_workspace_source_files(&workspace_root)?;
            let document_request =
                validate_document_symbol_batch_request(DocumentSymbolBatchRequest {
                    package_path: workspace_root.clone(),
                    workspace_root,
                    file_paths,
                })?;
            let workspace_root_uri = file_uri(&document_request.workspace_root)?;
            let db = resolve_cli_database_path(db, &config, &document_request.workspace_root)?;
            let store = start_writer(db, &config, &document_request.workspace_root).await?;

            let extraction = provider
                .extract_rust_references(ReferenceBatchRequest {
                    workspace_root: document_request.workspace_root,
                    package_path: document_request.package_path,
                    file_paths: document_request.file_paths,
                })
                .await?;
            let summary = ExtractionPersister
                .persist_reference_batch(&store, &workspace_root_uri, &extraction)
                .await?;
            shutdown_writer(&store).await?;

            println!(
                "workspace={} run={} targets={} references_edges={} reference_occurrences={} evidence={} stale_edges_closed={}",
                summary.workspace_id,
                summary.run_id,
                extraction.summary.targets_queried,
                summary.reference_edges,
                summary.reference_occurrences,
                summary.evidence,
                summary.stale_edges_closed
            );
        }
        Command::WorkspaceCalls { db, workspace_root } => {
            let provider = RustAnalyzerProvider::new();
            let file_paths = provider.discover_rust_workspace_source_files(&workspace_root)?;
            let document_request =
                validate_document_symbol_batch_request(DocumentSymbolBatchRequest {
                    package_path: workspace_root.clone(),
                    workspace_root,
                    file_paths,
                })?;
            let workspace_root_uri = file_uri(&document_request.workspace_root)?;
            let db = resolve_cli_database_path(db, &config, &document_request.workspace_root)?;
            let store = start_writer(db, &config, &document_request.workspace_root).await?;

            let extraction = provider
                .extract_rust_calls(CallBatchRequest {
                    workspace_root: document_request.workspace_root,
                    package_path: document_request.package_path,
                    file_paths: document_request.file_paths,
                })
                .await?;
            let summary = ExtractionPersister
                .persist_call_batch(&store, &workspace_root_uri, &extraction)
                .await?;
            shutdown_writer(&store).await?;

            println!(
                "workspace={} run={} callable_nodes={} calls_edges={} call_occurrences={} evidence={} skipped_external_targets={} skipped_unresolved_targets={} stale_edges_closed={}",
                summary.workspace_id,
                summary.run_id,
                extraction.summary.callable_nodes,
                summary.call_edges,
                summary.call_occurrences,
                summary.evidence,
                extraction.summary.skipped_external_targets,
                extraction.summary.skipped_unresolved_targets,
                summary.stale_edges_closed
            );
        }
        Command::WorkspaceAll {
            db,
            workspace_root,
            jobs,
            reference_jobs,
            call_jobs,
            analysis_workers,
            reference_analysis_workers,
            call_analysis_workers,
            serial,
        } => {
            let total_timer = Stopwatch::start_new();
            let mut benchmark = BenchmarkSummary::new();
            let provider = RustAnalyzerProvider::new();

            let discovery_timer = Stopwatch::start_new();
            let file_paths = provider.discover_rust_workspace_source_files(&workspace_root)?;
            benchmark.insert_duration_ms("discovery", discovery_timer.elapsed());
            benchmark.insert_count("files_discovered", file_paths.len());

            let request_validation_timer = Stopwatch::start_new();
            let document_request =
                validate_document_symbol_batch_request(DocumentSymbolBatchRequest {
                    package_path: workspace_root.clone(),
                    workspace_root,
                    file_paths,
                })?;
            let workspace_root_uri = file_uri(&document_request.workspace_root)?;
            benchmark.insert_duration_ms("request_validation", request_validation_timer.elapsed());

            let writer_ready_timer = Stopwatch::start_new();
            let db = resolve_cli_database_path(db, &config, &document_request.workspace_root)?;
            let store = start_writer(db, &config, &document_request.workspace_root).await?;
            benchmark.insert_duration_ms("writer_ready", writer_ready_timer.elapsed());

            let extractor_plan_timer = Stopwatch::start_new();
            let resolved_plan = resolve_cli_extractor_plan(CliExtractorPlanOptions {
                explicit_config_path: config.clone(),
                workspace_root: document_request.workspace_root.clone(),
                serial,
                jobs,
                reference_jobs,
                call_jobs,
                analysis_workers,
                reference_analysis_workers,
                call_analysis_workers,
            })?;
            benchmark.insert_duration_ms("extractor_plan", extractor_plan_timer.elapsed());
            benchmark.insert_label("mode", resolved_plan.mode.as_str());
            benchmark.insert_count("reference_jobs", resolved_plan.reference_jobs);
            benchmark.insert_count("call_jobs", resolved_plan.call_jobs);
            benchmark.insert_count("analysis_workers", resolved_plan.analysis_workers);
            benchmark.insert_count(
                "reference_analysis_workers",
                resolved_plan.reference_analysis_workers,
            );
            benchmark.insert_count("call_analysis_workers", resolved_plan.call_analysis_workers);

            if resolved_plan.mode == ExtractorMode::Threaded {
                let threaded_timer = Stopwatch::start_new();
                let summary = ThreadedWorkspaceAllRunner::run(
                    &store,
                    &provider,
                    document_request,
                    ThreadedWorkspaceAllConfig::new(
                        resolved_plan.reference_jobs,
                        resolved_plan.call_jobs,
                        resolved_plan.analysis_workers,
                        resolved_plan.reference_analysis_workers,
                        resolved_plan.call_analysis_workers,
                    ),
                )
                .await;
                benchmark.insert_duration_ms("threaded_runner", threaded_timer.elapsed());

                let writer_shutdown_timer = Stopwatch::start_new();
                shutdown_writer(&store).await?;
                benchmark.insert_duration_ms("writer_shutdown", writer_shutdown_timer.elapsed());
                let summary = summary?;
                benchmark.extend_from(&summary.benchmark);
                benchmark.insert_duration_ms("total", total_timer.elapsed());

                println!(
                    "workspace={} document_run={} reference_run={} call_run={} files={} nodes={} contains_edges={} references_edges={} reference_occurrences={} calls_edges={} call_occurrences={} evidence={} routes_complete={} stale_nodes_closed={} stale_edges_closed={}",
                    summary.document_summary.workspace_id,
                    summary.document_summary.run_id,
                    summary.reference_summary.run_id,
                    summary.call_summary.run_id,
                    summary.document_summary.files,
                    summary.document_summary.nodes,
                    summary.document_summary.edges,
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
                print_benchmark_summary(&benchmark);
                return Ok(());
            }

            let references_extract_timer = Stopwatch::start_new();
            let reference_extraction = provider
                .extract_rust_references(ReferenceBatchRequest {
                    workspace_root: document_request.workspace_root.clone(),
                    package_path: document_request.package_path.clone(),
                    file_paths: document_request.file_paths.clone(),
                })
                .await?;
            benchmark.insert_duration_ms(
                "serial.references_extract",
                references_extract_timer.elapsed(),
            );
            benchmark.insert_count(
                "serial.reference_targets",
                reference_extraction.summary.targets_queried,
            );

            let calls_extract_timer = Stopwatch::start_new();
            let call_extraction = provider
                .extract_rust_calls(CallBatchRequest {
                    workspace_root: document_request.workspace_root,
                    package_path: document_request.package_path,
                    file_paths: document_request.file_paths,
                })
                .await?;
            benchmark.insert_duration_ms("serial.calls_extract", calls_extract_timer.elapsed());
            benchmark.insert_count(
                "serial.call_targets",
                call_extraction.summary.callable_nodes,
            );
            benchmark.insert_count(
                "serial.document_files",
                reference_extraction.document_symbols.extractions.len(),
            );
            benchmark.insert_count(
                "serial.document_symbols",
                reference_extraction
                    .document_symbols
                    .extractions
                    .iter()
                    .map(|extraction| extraction.symbols.len())
                    .sum(),
            );

            let document_persist_timer = Stopwatch::start_new();
            let document_summary = ExtractionPersister
                .persist_document_symbol_batch(
                    &store,
                    &workspace_root_uri,
                    &reference_extraction.document_symbols,
                )
                .await?;
            benchmark
                .insert_duration_ms("serial.document_persist", document_persist_timer.elapsed());

            let references_persist_timer = Stopwatch::start_new();
            let reference_summary = ExtractionPersister
                .persist_reference_batch(&store, &workspace_root_uri, &reference_extraction)
                .await?;
            benchmark.insert_duration_ms(
                "serial.references_persist",
                references_persist_timer.elapsed(),
            );

            let calls_persist_timer = Stopwatch::start_new();
            let call_summary = ExtractionPersister
                .persist_call_batch(&store, &workspace_root_uri, &call_extraction)
                .await?;
            benchmark.insert_duration_ms("serial.calls_persist", calls_persist_timer.elapsed());

            let writer_shutdown_timer = Stopwatch::start_new();
            shutdown_writer(&store).await?;
            benchmark.insert_duration_ms("writer_shutdown", writer_shutdown_timer.elapsed());
            benchmark.insert_duration_ms("total", total_timer.elapsed());

            println!(
                "workspace={} document_run={} reference_run={} call_run={} files={} nodes={} contains_edges={} references_edges={} reference_occurrences={} calls_edges={} call_occurrences={} evidence={} routes_complete={} stale_nodes_closed={} stale_edges_closed={}",
                document_summary.workspace_id,
                document_summary.run_id,
                reference_summary.run_id,
                call_summary.run_id,
                document_summary.files,
                document_summary.nodes,
                document_summary.edges,
                reference_summary.reference_edges,
                reference_summary.reference_occurrences,
                call_summary.call_edges,
                call_summary.call_occurrences,
                document_summary.evidence + reference_summary.evidence + call_summary.evidence,
                document_summary.routes_complete
                    + reference_summary.routes_complete
                    + call_summary.routes_complete,
                document_summary.stale_nodes_closed,
                document_summary.stale_edges_closed
                    + reference_summary.stale_edges_closed
                    + call_summary.stale_edges_closed
            );
            print_benchmark_summary(&benchmark);
        }
    }

    Ok(())
}

fn resolve_rust_file_mode(
    calls: bool,
    references: bool,
    symbols: bool,
) -> ExtractResult<RustFileMode> {
    let selected = [calls, references, symbols]
        .into_iter()
        .filter(|selected| *selected)
        .count();
    if selected > 1 {
        return Err(ExtractError::response_shape(
            "rust-analyzer",
            "rust-file",
            "--calls, --references, and --symbols are mutually exclusive",
        ));
    }

    if calls {
        Ok(RustFileMode::Calls)
    } else if references {
        Ok(RustFileMode::References)
    } else if symbols {
        Ok(RustFileMode::Symbols)
    } else {
        Ok(RustFileMode::Full)
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

fn validate_deleted_rust_file_request(
    workspace_root: PathBuf,
    file_path: &Path,
) -> ExtractResult<(PathBuf, String, String)> {
    let workspace_root = workspace_root
        .canonicalize()
        .map_err(|source| ExtractError::io("canonicalize workspace root", None, source))?;
    let file_path = resolve_deleted_file_path(&workspace_root, file_path)?;

    if file_path.exists() && !file_path.is_file() {
        return Err(ExtractError::invalid_path(
            &file_path,
            &workspace_root,
            "deleted rust-file path must be a file path",
        ));
    }

    let relative_path = workspace_relative_path(&workspace_root, &file_path)?;
    let file_uri = file_uri(&file_path)?;

    Ok((workspace_root, file_uri, relative_path))
}

fn resolve_deleted_file_path(workspace_root: &Path, file_path: &Path) -> ExtractResult<PathBuf> {
    let file_path = if file_path.is_absolute() {
        file_path.to_path_buf()
    } else {
        workspace_root.join(file_path)
    };
    let file_path = normalize_lexical_path(&file_path);

    if !file_path.starts_with(workspace_root) {
        return Err(ExtractError::invalid_path(
            &file_path,
            workspace_root,
            "deleted rust-file path is outside the workspace root",
        ));
    }

    Ok(file_path)
}

fn normalize_lexical_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(value) => normalized.push(value),
        }
    }

    normalized
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

fn symbol_key_belongs_to_file(symbol_key: &str, file_scope_key: &str) -> bool {
    let file_node_key = format!("file:{file_scope_key}");
    symbol_key == file_node_key
        || symbol_key
            .strip_prefix(file_scope_key)
            .is_some_and(|suffix| suffix.starts_with('#'))
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

fn resolve_cli_extractor_plan(
    options: CliExtractorPlanOptions,
) -> ExtractResult<ResolvedExtractorPlan> {
    let extractor_config =
        resolve_cli_extractor_config(&options.explicit_config_path, &options.workspace_root)?;
    let mode = if options.serial {
        ExtractorMode::Serial
    } else if options.jobs.is_some()
        || options.reference_jobs.is_some()
        || options.call_jobs.is_some()
        || options.analysis_workers.is_some()
        || options.reference_analysis_workers.is_some()
        || options.call_analysis_workers.is_some()
    {
        ExtractorMode::Threaded
    } else {
        extractor_config.mode()
    };
    if mode == ExtractorMode::Serial {
        return Ok(ResolvedExtractorPlan {
            mode,
            reference_jobs: 1,
            call_jobs: 1,
            analysis_workers: 1,
            reference_analysis_workers: 0,
            call_analysis_workers: 0,
        });
    }
    let total_jobs = options
        .jobs
        .or_else(|| extractor_config.jobs())
        .unwrap_or_else(default_threaded_jobs);
    if total_jobs == 0 {
        return Err(ExtractError::response_shape(
            "rust-analyzer",
            "rust-workspace-all",
            "--jobs must be greater than zero",
        ));
    }

    let configured_reference_jobs = options
        .reference_jobs
        .or_else(|| extractor_config.reference_jobs());
    let configured_call_jobs = options.call_jobs.or_else(|| extractor_config.call_jobs());
    let (reference_jobs, call_jobs) =
        split_relation_jobs(total_jobs, configured_reference_jobs, configured_call_jobs)?;
    let analysis_workers = options
        .analysis_workers
        .or_else(|| extractor_config.analysis_workers())
        .unwrap_or(1);
    validate_single_route_jobs("--analysis-workers", analysis_workers)?;
    let reference_analysis_workers = options
        .reference_analysis_workers
        .or_else(|| extractor_config.reference_analysis_workers())
        .unwrap_or(0);
    let call_analysis_workers = options
        .call_analysis_workers
        .or_else(|| extractor_config.call_analysis_workers())
        .unwrap_or(0);

    Ok(ResolvedExtractorPlan {
        mode,
        reference_jobs,
        call_jobs,
        analysis_workers,
        reference_analysis_workers,
        call_analysis_workers,
    })
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

fn split_relation_jobs(
    total_jobs: usize,
    reference_jobs: Option<usize>,
    call_jobs: Option<usize>,
) -> ExtractResult<(usize, usize)> {
    match (reference_jobs, call_jobs) {
        (Some(reference_jobs), Some(call_jobs)) => {
            validate_relation_jobs(reference_jobs, call_jobs)?;
            Ok((reference_jobs, call_jobs))
        }
        (Some(reference_jobs), None) => {
            validate_single_route_jobs("--reference-jobs", reference_jobs)?;
            if reference_jobs >= total_jobs {
                return Err(invalid_worker_split(
                    "--reference-jobs must be less than --jobs when --call-jobs is omitted",
                ));
            }
            Ok((reference_jobs, total_jobs - reference_jobs))
        }
        (None, Some(call_jobs)) => {
            validate_single_route_jobs("--call-jobs", call_jobs)?;
            if call_jobs >= total_jobs {
                return Err(invalid_worker_split(
                    "--call-jobs must be less than --jobs when --reference-jobs is omitted",
                ));
            }
            Ok((total_jobs - call_jobs, call_jobs))
        }
        (None, None) => {
            if total_jobs < 2 {
                return Err(invalid_worker_split(
                    "--jobs must be at least 2 when threaded relation job counts are omitted",
                ));
            }
            let reference_jobs = total_jobs.div_ceil(2);
            Ok((reference_jobs, total_jobs - reference_jobs))
        }
    }
}

fn validate_relation_jobs(reference_jobs: usize, call_jobs: usize) -> ExtractResult<()> {
    validate_single_route_jobs("--reference-jobs", reference_jobs)?;
    validate_single_route_jobs("--call-jobs", call_jobs)
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
    ExtractError::response_shape("rust-analyzer", "rust-workspace-all", message)
}

fn print_benchmark_summary(summary: &BenchmarkSummary) {
    for line in summary.lines() {
        println!("{line}");
    }
}

fn default_threaded_jobs() -> usize {
    std::thread::available_parallelism()
        .map(|threads| threads.get().saturating_sub(1).max(2))
        .unwrap_or(2)
        .min(8)
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

fn resolve_cli_database_path(
    db: Option<PathBuf>,
    config: &Option<PathBuf>,
    workspace_root: &Path,
) -> ExtractResult<PathBuf> {
    resolve_database_path(LoadOptions {
        explicit_database_path: db,
        explicit_config_path: config.clone(),
        discovery_start_dir: Some(workspace_root.to_path_buf()),
        default_database_path: None,
    })
    .map(|resolved| resolved.into_path())
    .map_err(ExtractError::config)
}

#[cfg(test)]
mod cli_tests {
    use crate::{
        Cli, Command, RustFileMode, resolve_cli_database_path, resolve_rust_file_mode,
        validate_deleted_rust_file_request,
    };
    use clap::Parser;
    use std::{
        error::Error,
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn workspace_all_accepts_config_without_db() -> Result<(), Box<dyn Error>> {
        let cli = Cli::try_parse_from([
            "semantic-graph-extract",
            "rust-workspace-all",
            "--config",
            ".refactor-radar/config.toml",
            "--workspace-root",
            ".",
        ])?;

        assert_eq!(
            cli.config,
            Some(PathBuf::from(".refactor-radar/config.toml"))
        );
        match cli.command {
            Command::WorkspaceAll {
                db,
                workspace_root,
                jobs,
                reference_jobs,
                call_jobs,
                analysis_workers,
                reference_analysis_workers,
                call_analysis_workers,
                serial,
            } => {
                assert_eq!(db, None);
                assert_eq!(workspace_root, PathBuf::from("."));
                assert_eq!(jobs, None);
                assert_eq!(reference_jobs, None);
                assert_eq!(call_jobs, None);
                assert_eq!(analysis_workers, None);
                assert_eq!(reference_analysis_workers, None);
                assert_eq!(call_analysis_workers, None);
                assert!(!serial);
            }
            _ => return Err("expected rust-workspace-all command".into()),
        }

        Ok(())
    }

    #[test]
    fn rust_file_requires_only_file_and_defaults_workspace_root() -> Result<(), Box<dyn Error>> {
        let cli = Cli::try_parse_from([
            "semantic-graph-extract",
            "rust-file",
            "crates/semantic-graph-extract/src/main.rs",
        ])?;

        match cli.command {
            Command::RustFile {
                db,
                workspace_root,
                calls,
                references,
                symbols,
                file,
            } => {
                assert_eq!(db, None);
                assert_eq!(workspace_root, PathBuf::from("."));
                assert!(!calls);
                assert!(!references);
                assert!(!symbols);
                assert_eq!(
                    file,
                    PathBuf::from("crates/semantic-graph-extract/src/main.rs")
                );
            }
            _ => return Err("expected rust-file command".into()),
        }

        Ok(())
    }

    #[test]
    fn rust_file_accepts_workspace_root_and_symbols_mode() -> Result<(), Box<dyn Error>> {
        let cli = Cli::try_parse_from([
            "semantic-graph-extract",
            "rust-file",
            "--workspace-root",
            ".",
            "crates/semantic-graph-extract/src/main.rs",
            "--symbols",
        ])?;

        match cli.command {
            Command::RustFile {
                workspace_root,
                symbols,
                file,
                ..
            } => {
                assert_eq!(workspace_root, PathBuf::from("."));
                assert!(symbols);
                assert_eq!(
                    file,
                    PathBuf::from("crates/semantic-graph-extract/src/main.rs")
                );
            }
            _ => return Err("expected rust-file command".into()),
        }

        Ok(())
    }

    #[test]
    fn rust_file_modes_are_mutually_exclusive() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            resolve_rust_file_mode(false, false, false)?,
            RustFileMode::Full
        );
        assert_eq!(
            resolve_rust_file_mode(false, false, true)?,
            RustFileMode::Symbols
        );
        assert!(resolve_rust_file_mode(true, true, false).is_err());
        Ok(())
    }

    #[test]
    fn rust_file_deleted_defaults_workspace_root() -> Result<(), Box<dyn Error>> {
        let cli = Cli::try_parse_from([
            "semantic-graph-extract",
            "rust-file-deleted",
            "crates/wip/src/foo.rs",
        ])?;

        match cli.command {
            Command::RustFileDeleted {
                db,
                workspace_root,
                file,
            } => {
                assert_eq!(db, None);
                assert_eq!(workspace_root, PathBuf::from("."));
                assert_eq!(file, PathBuf::from("crates/wip/src/foo.rs"));
            }
            _ => return Err("expected rust-file-deleted command".into()),
        }

        Ok(())
    }

    #[test]
    fn rust_file_deleted_accepts_missing_file_path() -> Result<(), Box<dyn Error>> {
        let root = temp_dir("deleted-file-root")?;
        let deleted_file = PathBuf::from("crates/wip/src/deleted.rs");
        let (workspace_root, file_uri, relative_path) =
            validate_deleted_rust_file_request(root.clone(), &deleted_file)?;

        assert_eq!(workspace_root, root.canonicalize()?);
        assert!(file_uri.ends_with("/crates/wip/src/deleted.rs"));
        assert_eq!(relative_path, "crates/wip/src/deleted.rs");

        Ok(())
    }

    #[test]
    fn workspace_all_db_overrides_config() -> Result<(), Box<dyn Error>> {
        let root = temp_dir("extract-db-overrides-config")?;
        let config_path = write_config(&root, "path = \".local/config.db\"")?;
        let override_path = root.join("scratch.db");

        let resolved =
            resolve_cli_database_path(Some(override_path.clone()), &Some(config_path), &root)?;

        assert_eq!(resolved, override_path);
        Ok(())
    }

    #[test]
    fn workspace_all_discovers_config_from_workspace_root() -> Result<(), Box<dyn Error>> {
        let root = temp_dir("extract-discovers-config")?;
        let config_path = write_config(&root, "path = \".local/config.db\"")?;
        let workspace_subdirectory = root.join("crates/example");
        fs::create_dir_all(&workspace_subdirectory)?;

        let resolved = resolve_cli_database_path(None, &None, &workspace_subdirectory)?;

        assert_eq!(
            resolved,
            config_path
                .parent()
                .ok_or("expected config parent")?
                .join(".local/config.db")
        );
        Ok(())
    }

    fn write_config(root: &Path, database_line: &str) -> Result<PathBuf, Box<dyn Error>> {
        let config_dir = root.join(".refactor-radar");
        fs::create_dir_all(&config_dir)?;
        let config_path = config_dir.join("config.toml");
        fs::write(&config_path, format!("[database]\n{database_line}\n"))?;
        Ok(config_path)
    }

    fn temp_dir(name: &str) -> Result<PathBuf, Box<dyn Error>> {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = std::env::temp_dir().join(format!(
            "semantic-graph-extract-{name}-{}-{stamp}",
            std::process::id()
        ));
        fs::create_dir_all(&path)?;
        Ok(path)
    }
}
