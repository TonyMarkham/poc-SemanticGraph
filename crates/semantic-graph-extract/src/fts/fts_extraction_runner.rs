use crate::{
    ExtractError, ExtractResult,
    benchmark::{BenchmarkSummary, Stopwatch},
    document_symbols::paths::file_uri,
    fts::{
        FTS_PROVIDER, FTS_ROUTE, FTS_SCOPE, FtsDiscoveredFile, FtsExclusionSet,
        FtsExtractionOptions, FtsExtractionSummary, FtsFileDiscovery, FtsFileWorkResult,
        FtsFileWorkerJoinHandle, FtsFileWorkerMetric, FtsSkipReason, FtsStartedRun,
    },
};

use semantic_graph_config::FtsConfig;
use semantic_graph_db_manager::{
    CloseStaleFtsDocumentsInput, FtsWriteBatchDocumentInput, FtsWriteBatchInput,
    FtsWriteBatchSeenDocumentInput, RouteStatusCompleteInput, RouteStatusFailInput,
    RouteStatusStartInput, WriteHandle,
};
use serde_json::json;
use sha2::Digest;
use std::{
    collections::{HashMap, VecDeque},
    fs,
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::task::JoinError;

const MAX_INDEXED_FILE_BYTES: u64 = 10 * 1024 * 1024;

pub struct FtsExtractionRunner;

impl FtsExtractionRunner {
    pub async fn run(
        store: &WriteHandle,
        workspace_root: &Path,
        config: &FtsConfig,
        options: FtsExtractionOptions,
        analysis_workers: usize,
    ) -> ExtractResult<FtsExtractionSummary> {
        let total_timer = Stopwatch::start_new();
        let mut benchmark = BenchmarkSummary::new();
        benchmark.insert_label("fts.execution_mode", "file_content_index");
        benchmark.insert_label("fts.route", FTS_ROUTE);
        benchmark.insert_label("fts.scope", FTS_SCOPE);

        let canonicalize_timer = Stopwatch::start_new();
        let workspace_root = workspace_root
            .canonicalize()
            .map_err(|source| ExtractError::io("canonicalize FTS workspace root", None, source))?;
        benchmark.insert_duration_ms(
            "fts.workspace_root_canonicalize",
            canonicalize_timer.elapsed(),
        );

        let workspace_root_uri_timer = Stopwatch::start_new();
        let workspace_root_uri = file_uri(&workspace_root)?;
        benchmark.insert_duration_ms("fts.workspace_root_uri", workspace_root_uri_timer.elapsed());

        let create_workspace_timer = Stopwatch::start_new();
        let workspace_id = store
            .create_workspace(&workspace_root_uri, "mixed")
            .await
            .map_err(ExtractError::storage)?;
        benchmark.insert_duration_ms("fts.create_workspace", create_workspace_timer.elapsed());

        let start_run_timer = Stopwatch::start_new();
        let run_id = store
            .start_run(workspace_id, FTS_PROVIDER, None, None)
            .await
            .map_err(ExtractError::storage)?;
        benchmark.insert_duration_ms("fts.start_run", start_run_timer.elapsed());

        let result = Self::run_started(
            store,
            &workspace_root,
            FtsStartedRun {
                workspace_root_uri: workspace_root_uri.clone(),
                workspace_id,
                run_id,
                analysis_workers,
            },
            config,
            options,
        )
        .await;

        if result.is_err() {
            let _fail_route_result = store
                .fail_route_status(RouteStatusFailInput {
                    workspace_id,
                    route: FTS_ROUTE,
                    scope: FTS_SCOPE,
                    scope_key: &workspace_root_uri,
                    provider: FTS_PROVIDER,
                    run_id,
                    diagnostics_json: json!({ "status": "failed" }),
                })
                .await;
            let _finish_run_result = store.finish_run(run_id, "failed").await;
        }

        let mut summary = result?;
        benchmark.extend_from(&summary.benchmark);
        benchmark.insert_duration_ms("fts.total", total_timer.elapsed());
        summary.benchmark = benchmark;

        Ok(summary)
    }

    async fn run_started(
        store: &WriteHandle,
        workspace_root: &Path,
        started_run: FtsStartedRun,
        config: &FtsConfig,
        options: FtsExtractionOptions,
    ) -> ExtractResult<FtsExtractionSummary> {
        let analysis_workers = started_run.analysis_workers.max(1);
        let mut benchmark = BenchmarkSummary::new();
        let route_start_timer = Stopwatch::start_new();
        store
            .start_route_status(RouteStatusStartInput {
                workspace_id: started_run.workspace_id,
                route: FTS_ROUTE,
                scope: FTS_SCOPE,
                scope_key: &started_run.workspace_root_uri,
                file_id: None,
                provider: FTS_PROVIDER,
                provider_version: None,
                content_hash: None,
                run_id: started_run.run_id,
                diagnostics_json: json!({ "status": "running" }),
            })
            .await
            .map_err(ExtractError::storage)?;
        benchmark.insert_duration_ms("fts.route_start", route_start_timer.elapsed());

        let discovery_timer = Stopwatch::start_new();
        let exclusions = FtsExclusionSet::new(workspace_root, config, options)?;
        let discovery = FtsFileDiscovery::discover(workspace_root, &exclusions)?;
        benchmark.insert_duration_ms("fts.discovery", discovery_timer.elapsed());
        let mut summary = FtsExtractionSummary {
            benchmark: BenchmarkSummary::new(),
            workspace_id: started_run.workspace_id,
            run_id: started_run.run_id,
            scanned_files: discovery.scanned_files(),
            files_hashed: 0,
            files_hash_unchanged: 0,
            files_changed: 0,
            indexed_files: 0,
            skipped_files: discovery.skipped_files(),
            skipped_directories: discovery.skipped_directories(),
            skipped_by_config: discovery.skipped_by_config(),
            skipped_by_no_rust: discovery.skipped_by_no_rust(),
            skipped_by_no_csharp: discovery.skipped_by_no_csharp(),
            skipped_by_no_submodules: discovery.skipped_by_no_submodules(),
            skipped_binary_or_unreadable: 0,
            stale_fts_documents_closed: 0,
        };
        benchmark.insert_count("fts.discovered_files", discovery.files().len());
        benchmark.insert_count("fts.scanned_files", summary.scanned_files);
        benchmark.insert_count("fts.skipped_files", summary.skipped_files);
        benchmark.insert_count("fts.skipped_directories", summary.skipped_directories);
        benchmark.insert_count("fts.skipped_by_config", summary.skipped_by_config);
        benchmark.insert_count("fts.skipped_by_no_rust", summary.skipped_by_no_rust);
        benchmark.insert_count("fts.skipped_by_no_csharp", summary.skipped_by_no_csharp);
        benchmark.insert_count(
            "fts.skipped_by_no_submodules",
            summary.skipped_by_no_submodules,
        );

        let active_hash_lookup_timer = Stopwatch::start_new();
        let active_fts_document_hashes = store
            .active_fts_document_hashes(started_run.workspace_id)
            .await
            .map_err(ExtractError::storage)?;
        benchmark.insert_duration_ms("fts.active_hash_lookup", active_hash_lookup_timer.elapsed());
        benchmark.insert_count("fts.active_hashes", active_fts_document_hashes.len());

        let mut batch = FtsWriteBatchInput::default();
        let mut fingerprint_entries = Vec::new();
        let mut indexed_bytes = 0usize;
        let mut file_read_elapsed = Duration::from_millis(0);
        let mut file_hash_elapsed = Duration::from_millis(0);
        let mut file_uri_elapsed = Duration::from_millis(0);

        let file_processing_timer = Stopwatch::start_new();
        let (file_results, worker_metrics) = run_fts_file_workers(
            discovery.files().to_vec(),
            active_fts_document_hashes,
            started_run.workspace_id,
            started_run.run_id,
            analysis_workers,
            FTS_ROUTE,
        )
        .await?;
        for result in file_results {
            file_read_elapsed += result.file_read_elapsed;
            file_hash_elapsed += result.file_hash_elapsed;
            file_uri_elapsed += result.file_uri_elapsed;
            if let Some(reason) = result.skip_reason {
                summary.count_runtime_skip(reason);
            }
            if let Some(fingerprint_entry) = result.fingerprint_entry {
                fingerprint_entries.push(fingerprint_entry);
                summary.files_hashed += 1;
            }
            if let Some(seen_document) = result.seen_document {
                batch.seen_documents.push(seen_document);
                summary.files_hash_unchanged += 1;
            }
            if let Some(document) = result.document {
                batch.documents.push(document);
                summary.count_indexed_file();
                summary.files_changed += 1;
                indexed_bytes += result.indexed_bytes;
            }
        }
        benchmark.insert_duration_ms("fts.file_processing", file_processing_timer.elapsed());
        insert_fts_file_worker_metrics(&mut benchmark, &worker_metrics);
        benchmark.insert_count("fts.analysis_workers", analysis_workers);
        benchmark.insert_duration_ms("fts.file_read", file_read_elapsed);
        benchmark.insert_duration_ms("fts.file_hash", file_hash_elapsed);
        benchmark.insert_duration_ms("fts.file_uri", file_uri_elapsed);
        let write_batch_timer = Stopwatch::start_new();
        store
            .write_fts_batch(batch)
            .await
            .map_err(ExtractError::storage)?;
        benchmark.insert_duration_ms("fts.write_batch", write_batch_timer.elapsed());
        benchmark.insert_label("fts.write_mode", "fts_write_batch");
        benchmark.insert_count("fts.files_hashed", summary.files_hashed);
        benchmark.insert_count("fts.files_hash_unchanged", summary.files_hash_unchanged);
        benchmark.insert_count("fts.files_changed", summary.files_changed);
        benchmark.insert_count("fts.indexed_files", summary.indexed_files);
        benchmark.insert_count("fts.skipped_files", summary.skipped_files);
        benchmark.insert_count(
            "fts.skipped_binary_or_unreadable",
            summary.skipped_binary_or_unreadable,
        );
        benchmark.insert_count("fts.indexed_bytes", indexed_bytes);
        benchmark.insert_count("fts.fingerprint_entries", fingerprint_entries.len());

        let route_content_hash_timer = Stopwatch::start_new();
        let route_content_hash = route_content_hash(fingerprint_entries);
        benchmark.insert_duration_ms("fts.route_content_hash", route_content_hash_timer.elapsed());
        let diagnostics_json = json!({
            "scanned_files": summary.scanned_files,
            "files_hashed": summary.files_hashed,
            "files_hash_unchanged": summary.files_hash_unchanged,
            "files_changed": summary.files_changed,
            "indexed_files": summary.indexed_files,
            "skipped_files": summary.skipped_files,
            "skipped_directories": summary.skipped_directories,
            "skipped_by_config": summary.skipped_by_config,
            "skipped_by_no_rust": summary.skipped_by_no_rust,
            "skipped_by_no_csharp": summary.skipped_by_no_csharp,
            "skipped_by_no_submodules": summary.skipped_by_no_submodules,
            "skipped_binary_or_unreadable": summary.skipped_binary_or_unreadable,
        });
        let route_complete_timer = Stopwatch::start_new();
        store
            .complete_route_status(RouteStatusCompleteInput {
                workspace_id: started_run.workspace_id,
                route: FTS_ROUTE,
                scope: FTS_SCOPE,
                scope_key: &started_run.workspace_root_uri,
                provider: FTS_PROVIDER,
                provider_version: None,
                content_hash: Some(&route_content_hash),
                run_id: started_run.run_id,
                diagnostics_json,
            })
            .await
            .map_err(ExtractError::storage)?;
        benchmark.insert_duration_ms("fts.route_complete", route_complete_timer.elapsed());

        let close_stale_timer = Stopwatch::start_new();
        summary.stale_fts_documents_closed = store
            .close_stale_fts_documents_for_workspace(CloseStaleFtsDocumentsInput {
                workspace_id: started_run.workspace_id,
                run_id: started_run.run_id,
                provider: FTS_PROVIDER,
                route: FTS_ROUTE,
                scope: FTS_SCOPE,
                scope_key: &started_run.workspace_root_uri,
            })
            .await
            .map_err(ExtractError::storage)?;
        benchmark.insert_duration_ms("fts.close_stale_documents", close_stale_timer.elapsed());
        benchmark.insert_label(
            "fts.stale_fts_documents_closed",
            summary.stale_fts_documents_closed.to_string(),
        );

        let finish_run_timer = Stopwatch::start_new();
        store
            .finish_run(started_run.run_id, "complete")
            .await
            .map_err(ExtractError::storage)?;
        benchmark.insert_duration_ms("fts.finish_run", finish_run_timer.elapsed());

        summary.benchmark = benchmark;

        Ok(summary)
    }
}

pub(crate) async fn run_fts_file_workers(
    files: Vec<FtsDiscoveredFile>,
    active_fts_document_hashes: HashMap<String, String>,
    workspace_id: i64,
    run_id: i64,
    analysis_workers: usize,
    route: &'static str,
) -> ExtractResult<(Vec<FtsFileWorkResult>, Vec<FtsFileWorkerMetric>)> {
    if files.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }

    let worker_count = analysis_workers.max(1).min(files.len());
    let queue = Arc::new(Mutex::new(VecDeque::from(files)));
    let active_hashes = Arc::new(active_fts_document_hashes);
    let failed = Arc::new(AtomicBool::new(false));
    let mut handles = Vec::with_capacity(worker_count);

    for worker_index in 0..worker_count {
        let worker_queue = Arc::clone(&queue);
        let worker_active_hashes = Arc::clone(&active_hashes);
        let worker_failed = Arc::clone(&failed);
        let worker_failed_for_result = Arc::clone(&worker_failed);
        handles.push(tokio::task::spawn_blocking(move || {
            let result = fts_file_worker(
                worker_index,
                worker_queue,
                worker_active_hashes,
                worker_failed,
                workspace_id,
                run_id,
                route,
            );
            if result.is_err() {
                worker_failed_for_result.store(true, Ordering::SeqCst);
            }
            result
        }));
    }

    collect_fts_file_workers(handles).await
}

fn fts_file_worker(
    worker_index: usize,
    queue: Arc<Mutex<VecDeque<FtsDiscoveredFile>>>,
    active_fts_document_hashes: Arc<HashMap<String, String>>,
    failed: Arc<AtomicBool>,
    workspace_id: i64,
    run_id: i64,
    route: &'static str,
) -> ExtractResult<(Vec<FtsFileWorkResult>, FtsFileWorkerMetric)> {
    let timer = Stopwatch::start_new();
    let mut results = Vec::new();
    let mut files = 0;
    let mut files_hashed = 0;
    let mut files_changed = 0;
    let mut files_hash_unchanged = 0;
    let mut skipped_binary_or_unreadable = 0;

    loop {
        if failed.load(Ordering::SeqCst) {
            return Ok((
                results,
                FtsFileWorkerMetric {
                    worker_index,
                    files,
                    files_hashed,
                    files_changed,
                    files_hash_unchanged,
                    skipped_binary_or_unreadable,
                    elapsed: timer.elapsed(),
                },
            ));
        }

        let file = {
            let mut queue = queue.lock().map_err(|error| {
                ExtractError::process(
                    "semantic-graph-extract",
                    "fts worker queue",
                    error.to_string(),
                )
            })?;
            queue.pop_front()
        };
        let Some(file) = file else {
            return Ok((
                results,
                FtsFileWorkerMetric {
                    worker_index,
                    files,
                    files_hashed,
                    files_changed,
                    files_hash_unchanged,
                    skipped_binary_or_unreadable,
                    elapsed: timer.elapsed(),
                },
            ));
        };

        files += 1;
        let result = process_fts_file(
            &file,
            &active_fts_document_hashes,
            workspace_id,
            run_id,
            route,
        )?;
        if result.fingerprint_entry.is_some() {
            files_hashed += 1;
        }
        if result.document.is_some() {
            files_changed += 1;
        }
        if result.seen_document.is_some() {
            files_hash_unchanged += 1;
        }
        if result.skip_reason == Some(FtsSkipReason::BinaryOrUnreadable) {
            skipped_binary_or_unreadable += 1;
        }
        results.push(result);
    }
}

async fn collect_fts_file_workers(
    handles: Vec<FtsFileWorkerJoinHandle>,
) -> ExtractResult<(Vec<FtsFileWorkResult>, Vec<FtsFileWorkerMetric>)> {
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
                    first_error = Some(worker_join_error("fts.file_processing", error));
                }
            }
        }
    }

    match first_error {
        Some(error) => Err(error),
        None => {
            worker_metrics.sort_by_key(|metric| metric.worker_index);
            Ok((file_results, worker_metrics))
        }
    }
}

fn process_fts_file(
    file: &FtsDiscoveredFile,
    active_fts_document_hashes: &HashMap<String, String>,
    workspace_id: i64,
    run_id: i64,
    route: &str,
) -> ExtractResult<FtsFileWorkResult> {
    let file_read_timer = Stopwatch::start_new();
    let text = match read_indexable_text(file.absolute_path())? {
        Ok(text) => text,
        Err(reason) => {
            return Ok(FtsFileWorkResult::skipped(
                reason,
                file_read_timer.elapsed(),
            ));
        }
    };
    let file_read_elapsed = file_read_timer.elapsed();

    let file_hash_timer = Stopwatch::start_new();
    let content_hash = sha256_hex(text.as_bytes());
    let file_hash_elapsed = file_hash_timer.elapsed();
    let text_len = text.len();

    let file_uri_timer = Stopwatch::start_new();
    let file_uri = file_uri(file.absolute_path())?;
    let file_uri_elapsed = file_uri_timer.elapsed();
    let fingerprint_entry = Some(format!("{}:{content_hash}", file.relative_path()));

    if active_fts_document_hashes.get(&file_uri) == Some(&content_hash) {
        return Ok(FtsFileWorkResult {
            document: None,
            seen_document: Some(FtsWriteBatchSeenDocumentInput {
                workspace_id,
                uri: file_uri,
                content_hash,
                run_id,
            }),
            fingerprint_entry,
            skip_reason: None,
            indexed_bytes: 0,
            file_read_elapsed,
            file_hash_elapsed,
            file_uri_elapsed,
        });
    }

    Ok(FtsFileWorkResult {
        document: Some(FtsWriteBatchDocumentInput {
            workspace_id,
            uri: file_uri,
            path: file.relative_path().to_string(),
            language: file.language().as_str().to_string(),
            content_hash,
            byte_len: text_len as i64,
            run_id,
            content: text,
            properties_json: json!({ "route": route }),
        }),
        seen_document: None,
        fingerprint_entry,
        skip_reason: None,
        indexed_bytes: text_len,
        file_read_elapsed,
        file_hash_elapsed,
        file_uri_elapsed,
    })
}

pub(crate) fn insert_fts_file_worker_metrics(
    benchmark: &mut BenchmarkSummary,
    worker_metrics: &[FtsFileWorkerMetric],
) {
    insert_fts_file_worker_metrics_with_prefix(benchmark, "fts", worker_metrics);
}

pub(crate) fn insert_fts_file_worker_metrics_with_prefix(
    benchmark: &mut BenchmarkSummary,
    prefix: &str,
    worker_metrics: &[FtsFileWorkerMetric],
) {
    benchmark.insert_count(&format!("{prefix}.file_workers"), worker_metrics.len());
    benchmark.insert_count(
        &format!("{prefix}.file_active_workers"),
        worker_metrics
            .iter()
            .filter(|metric| metric.files > 0)
            .count(),
    );

    for metric in worker_metrics {
        let worker_prefix = format!("{prefix}.worker.{}", metric.worker_index);
        benchmark.insert_count(&format!("{worker_prefix}.files"), metric.files);
        benchmark.insert_count(
            &format!("{worker_prefix}.files_hashed"),
            metric.files_hashed,
        );
        benchmark.insert_count(
            &format!("{worker_prefix}.files_changed"),
            metric.files_changed,
        );
        benchmark.insert_count(
            &format!("{worker_prefix}.files_hash_unchanged"),
            metric.files_hash_unchanged,
        );
        benchmark.insert_count(
            &format!("{worker_prefix}.skipped_binary_or_unreadable"),
            metric.skipped_binary_or_unreadable,
        );
        benchmark.insert_duration_ms(&format!("{worker_prefix}.elapsed"), metric.elapsed);
    }
}

fn read_indexable_text(path: &Path) -> ExtractResult<Result<String, FtsSkipReason>> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(_error) => {
            return Ok(Err(FtsSkipReason::BinaryOrUnreadable));
        }
    };
    if metadata.len() > MAX_INDEXED_FILE_BYTES {
        return Ok(Err(FtsSkipReason::BinaryOrUnreadable));
    }

    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(_error) => {
            return Ok(Err(FtsSkipReason::BinaryOrUnreadable));
        }
    };
    if bytes.contains(&0) {
        return Ok(Err(FtsSkipReason::BinaryOrUnreadable));
    }

    match String::from_utf8(bytes) {
        Ok(text) => Ok(Ok(text)),
        Err(_error) => Ok(Err(FtsSkipReason::BinaryOrUnreadable)),
    }
}

fn worker_join_error(route: &str, error: JoinError) -> ExtractError {
    ExtractError::process("semantic-graph-extract", route, error.to_string())
}

pub(crate) fn route_content_hash(mut entries: Vec<String>) -> String {
    entries.sort();
    let mut hasher = sha2::Sha256::new();
    for entry in entries {
        hasher.update(entry.as_bytes());
        hasher.update(b"\n");
    }

    hex::encode(hasher.finalize())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}
