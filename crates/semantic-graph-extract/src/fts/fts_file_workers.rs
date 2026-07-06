use crate::{
    ExtractError, ExtractResult,
    benchmark::{BenchmarkSummary, Stopwatch},
    document_symbols::paths::file_uri,
    fts::{
        FtsDiscoveredFile, FtsFileWorkResult, FtsFileWorkerConfig, FtsFileWorkerInput,
        FtsFileWorkerJoinHandle, FtsFileWorkerMetric, FtsSkipReason,
    },
    progress::ProgressTask,
};

use semantic_graph_db_manager::{FtsWriteBatchDocumentInput, FtsWriteBatchSeenDocumentInput};
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
};
use tokio::task::JoinError;

pub(crate) async fn run_fts_file_workers_with_progress(
    input: FtsFileWorkerInput,
    progress: ProgressTask,
) -> ExtractResult<(Vec<FtsFileWorkResult>, Vec<FtsFileWorkerMetric>)> {
    if input.files.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }

    let worker_count = input.analysis_workers.max(1).min(input.files.len());
    let queue = Arc::new(Mutex::new(VecDeque::from(input.files)));
    let active_hashes = Arc::new(input.active_fts_document_hashes);
    let worker_config = FtsFileWorkerConfig {
        workspace_id: input.workspace_id,
        run_id: input.run_id,
        max_indexed_file_bytes: input.max_indexed_file_bytes,
        route: input.route,
    };
    let failed = Arc::new(AtomicBool::new(false));
    let mut handles = Vec::with_capacity(worker_count);

    for worker_index in 0..worker_count {
        let worker_queue = Arc::clone(&queue);
        let worker_active_hashes = Arc::clone(&active_hashes);
        let worker_failed = Arc::clone(&failed);
        let worker_failed_for_result = Arc::clone(&worker_failed);
        let worker_progress = progress.clone();
        handles.push(tokio::task::spawn_blocking(move || {
            let result = fts_file_worker(
                worker_index,
                worker_queue,
                worker_active_hashes,
                worker_failed,
                worker_config,
                worker_progress,
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
    worker_config: FtsFileWorkerConfig,
    progress: ProgressTask,
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
        let result = process_fts_file(&file, &active_fts_document_hashes, worker_config)?;
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
        progress.tick();
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
    worker_config: FtsFileWorkerConfig,
) -> ExtractResult<FtsFileWorkResult> {
    let file_read_timer = Stopwatch::start_new();
    let text =
        match read_indexable_text(file.absolute_path(), worker_config.max_indexed_file_bytes)? {
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
                workspace_id: worker_config.workspace_id,
                uri: file_uri,
                content_hash,
                run_id: worker_config.run_id,
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
            workspace_id: worker_config.workspace_id,
            uri: file_uri,
            path: file.relative_path().to_string(),
            language: file.language().as_str().to_string(),
            content_hash,
            byte_len: text_len as i64,
            run_id: worker_config.run_id,
            content: text,
            properties_json: json!({ "route": worker_config.route }),
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

fn read_indexable_text(
    path: &Path,
    max_indexed_file_bytes: u64,
) -> ExtractResult<Result<String, FtsSkipReason>> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(_error) => {
            return Ok(Err(FtsSkipReason::BinaryOrUnreadable));
        }
    };
    if metadata.len() > max_indexed_file_bytes {
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
