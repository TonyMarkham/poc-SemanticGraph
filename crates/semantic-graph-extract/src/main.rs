use semantic_graph_extract::{
    CliExtractorPlanOptions, ExtractError, ExtractResult, ResolvedExtractorPlan,
    benchmark::{BenchmarkSummary, Stopwatch},
    document_symbols::paths::{
        file_uri, validate_document_symbol_batch_request, validate_document_symbol_request,
    },
    model::{
        CallBatchRequest, DocumentSymbolBatchRequest, DocumentSymbolRequest, ReferenceBatchRequest,
    },
    persist::ExtractionPersister,
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
    path::{Path, PathBuf},
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
    use crate::{Cli, Command, resolve_cli_database_path};
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
