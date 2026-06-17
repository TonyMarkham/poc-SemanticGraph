use crate::{
    ExtractError, ExtractResult,
    benchmark::{BenchmarkSummary, Stopwatch},
    document_symbols::paths::file_uri,
    fts::{
        FTS_PROVIDER, FTS_SCOPE, FTS_TANTIVY_ROUTE, FtsExclusionSet, FtsExtractionOptions,
        FtsExtractionSummary, FtsFileDiscovery, FtsStartedRun,
        insert_fts_file_worker_metrics_with_prefix, route_content_hash, run_fts_file_workers,
    },
};

use semantic_graph_config::FtsConfig;
use semantic_graph_db_manager::{
    CloseStaleFtsDocumentsInput, FtsWriteBatchInput, RouteStatusCompleteInput,
    RouteStatusFailInput, RouteStatusStartInput, WriteHandle,
};
use semantic_graph_search_tantivy::{TantivyFtsDocument, TantivyFtsIndex, TantivyFtsIndexUpdate};
use serde_json::json;
use std::{
    collections::{HashMap, HashSet},
    path::{Component, Path},
    time::Duration,
};

pub struct FtsTantivyExtractionRunner;

impl FtsTantivyExtractionRunner {
    pub async fn run(
        store: &WriteHandle,
        workspace_root: &Path,
        db_path: &Path,
        index_path: &Path,
        config: &FtsConfig,
        options: FtsExtractionOptions,
        analysis_workers: usize,
    ) -> ExtractResult<FtsExtractionSummary> {
        let total_timer = Stopwatch::start_new();
        let mut benchmark = BenchmarkSummary::new();
        benchmark.insert_label("fts_tantivy.execution_mode", "file_content_tantivy_index");
        benchmark.insert_label("fts_tantivy.route", FTS_TANTIVY_ROUTE);
        benchmark.insert_label("fts_tantivy.scope", FTS_SCOPE);

        let canonicalize_timer = Stopwatch::start_new();
        let workspace_root = workspace_root
            .canonicalize()
            .map_err(|source| ExtractError::io("canonicalize FTS workspace root", None, source))?;
        benchmark.insert_duration_ms(
            "fts_tantivy.workspace_root_canonicalize",
            canonicalize_timer.elapsed(),
        );

        let workspace_root_uri_timer = Stopwatch::start_new();
        let workspace_root_uri = file_uri(&workspace_root)?;
        benchmark.insert_duration_ms(
            "fts_tantivy.workspace_root_uri",
            workspace_root_uri_timer.elapsed(),
        );

        let create_workspace_timer = Stopwatch::start_new();
        let workspace_id = store
            .create_workspace(&workspace_root_uri, "mixed")
            .await
            .map_err(ExtractError::storage)?;
        benchmark.insert_duration_ms(
            "fts_tantivy.create_workspace",
            create_workspace_timer.elapsed(),
        );

        let start_run_timer = Stopwatch::start_new();
        let run_id = store
            .start_run(workspace_id, FTS_PROVIDER, None, None)
            .await
            .map_err(ExtractError::storage)?;
        benchmark.insert_duration_ms("fts_tantivy.start_run", start_run_timer.elapsed());

        let result = Self::run_started(
            store,
            &workspace_root,
            db_path,
            index_path,
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
                    route: FTS_TANTIVY_ROUTE,
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
        benchmark.insert_duration_ms("fts_tantivy.total", total_timer.elapsed());
        summary.benchmark = benchmark;

        Ok(summary)
    }

    async fn run_started(
        store: &WriteHandle,
        workspace_root: &Path,
        db_path: &Path,
        index_path: &Path,
        started_run: FtsStartedRun,
        config: &FtsConfig,
        options: FtsExtractionOptions,
    ) -> ExtractResult<FtsExtractionSummary> {
        let analysis_workers = started_run.analysis_workers.max(1);
        let mut benchmark = BenchmarkSummary::new();
        benchmark.insert_count("fts_tantivy.analysis_workers", analysis_workers);
        benchmark.insert_label(
            "fts_tantivy.index_path",
            index_path.to_string_lossy().to_string(),
        );

        let route_start_timer = Stopwatch::start_new();
        store
            .start_route_status(RouteStatusStartInput {
                workspace_id: started_run.workspace_id,
                route: FTS_TANTIVY_ROUTE,
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
        benchmark.insert_duration_ms("fts_tantivy.route_start", route_start_timer.elapsed());

        let discovery_timer = Stopwatch::start_new();
        let (runtime_excluded_directories, runtime_excluded_files) =
            runtime_artifact_exclusions(workspace_root, db_path, index_path);
        benchmark.insert_count(
            "fts_tantivy.runtime_excluded_directories",
            runtime_excluded_directories.len(),
        );
        benchmark.insert_count(
            "fts_tantivy.runtime_excluded_files",
            runtime_excluded_files.len(),
        );
        let exclusions = FtsExclusionSet::new(workspace_root, config, options)?
            .with_runtime_exclusions(runtime_excluded_directories, runtime_excluded_files);
        let discovery = FtsFileDiscovery::discover(workspace_root, &exclusions)?;
        benchmark.insert_duration_ms("fts_tantivy.discovery", discovery_timer.elapsed());
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
        benchmark.insert_count("fts_tantivy.discovered_files", discovery.files().len());
        benchmark.insert_count("fts_tantivy.scanned_files", summary.scanned_files);
        benchmark.insert_count("fts_tantivy.skipped_files", summary.skipped_files);
        benchmark.insert_count(
            "fts_tantivy.skipped_directories",
            summary.skipped_directories,
        );
        benchmark.insert_count("fts_tantivy.skipped_by_config", summary.skipped_by_config);
        benchmark.insert_count("fts_tantivy.skipped_by_no_rust", summary.skipped_by_no_rust);
        benchmark.insert_count(
            "fts_tantivy.skipped_by_no_csharp",
            summary.skipped_by_no_csharp,
        );
        benchmark.insert_count(
            "fts_tantivy.skipped_by_no_submodules",
            summary.skipped_by_no_submodules,
        );

        let active_hash_lookup_timer = Stopwatch::start_new();
        let active_fts_document_hashes = store
            .active_fts_document_hashes(started_run.workspace_id)
            .await
            .map_err(ExtractError::storage)?;
        benchmark.insert_duration_ms(
            "fts_tantivy.active_hash_lookup",
            active_hash_lookup_timer.elapsed(),
        );
        benchmark.insert_count(
            "fts_tantivy.active_hashes",
            active_fts_document_hashes.len(),
        );

        let index_exists = index_path.join("meta.json").exists();
        benchmark.insert_label("fts_tantivy.index_exists", index_exists.to_string());
        let active_document_uris = active_fts_document_hashes
            .keys()
            .cloned()
            .collect::<HashSet<_>>();
        let worker_active_hashes = if index_exists {
            active_fts_document_hashes
        } else {
            HashMap::new()
        };

        let mut batch = FtsWriteBatchInput::default();
        let mut tantivy_documents = Vec::new();
        let mut current_document_uris = HashSet::new();
        let mut fingerprint_entries = Vec::new();
        let mut indexed_bytes = 0usize;
        let mut file_read_elapsed = Duration::from_millis(0);
        let mut file_hash_elapsed = Duration::from_millis(0);
        let mut file_uri_elapsed = Duration::from_millis(0);

        let file_processing_timer = Stopwatch::start_new();
        let (file_results, worker_metrics) = run_fts_file_workers(
            discovery.files().to_vec(),
            worker_active_hashes,
            started_run.workspace_id,
            started_run.run_id,
            analysis_workers,
            FTS_TANTIVY_ROUTE,
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
                current_document_uris.insert(seen_document.uri.clone());
                batch.seen_documents.push(seen_document);
                summary.files_hash_unchanged += 1;
            }
            if let Some(document) = result.document {
                current_document_uris.insert(document.uri.clone());
                tantivy_documents.push(TantivyFtsDocument {
                    uri: document.uri.clone(),
                    path: document.path.clone(),
                    language: document.language.clone(),
                    content_hash: document.content_hash.clone(),
                    content: document.content.clone(),
                });
                batch.documents.push(document);
                summary.count_indexed_file();
                summary.files_changed += 1;
                indexed_bytes += result.indexed_bytes;
            }
        }
        benchmark.insert_duration_ms(
            "fts_tantivy.file_processing",
            file_processing_timer.elapsed(),
        );
        insert_fts_file_worker_metrics_with_prefix(&mut benchmark, "fts_tantivy", &worker_metrics);
        benchmark.insert_duration_ms("fts_tantivy.file_read", file_read_elapsed);
        benchmark.insert_duration_ms("fts_tantivy.file_hash", file_hash_elapsed);
        benchmark.insert_duration_ms("fts_tantivy.file_uri", file_uri_elapsed);

        let mut deleted_uris = active_document_uris
            .difference(&current_document_uris)
            .cloned()
            .collect::<Vec<_>>();
        deleted_uris.sort();

        let tantivy_update_timer = Stopwatch::start_new();
        let tantivy_index = TantivyFtsIndex::open_or_create(index_path)
            .map_err(|source| ExtractError::tantivy_search("open fts tantivy index", source))?;
        let tantivy_update_summary = tantivy_index
            .apply_update(TantivyFtsIndexUpdate {
                documents: tantivy_documents,
                deleted_uris,
                indexing_workers: analysis_workers,
            })
            .map_err(|source| ExtractError::tantivy_search("update fts tantivy index", source))?;
        benchmark.insert_duration_ms("fts_tantivy.index_update", tantivy_update_timer.elapsed());
        benchmark.insert_count(
            "fts_tantivy.indexed_documents",
            tantivy_update_summary.indexed_documents,
        );
        benchmark.insert_count(
            "fts_tantivy.deleted_uris",
            tantivy_update_summary.deleted_uris,
        );
        benchmark.insert_label(
            "fts_tantivy.index_committed",
            tantivy_update_summary.committed.to_string(),
        );
        benchmark.insert_count(
            "fts_tantivy.indexing_workers",
            tantivy_update_summary.indexing_workers,
        );
        benchmark.insert_count(
            "fts_tantivy.index_memory_budget_bytes",
            tantivy_update_summary.memory_budget_bytes,
        );

        let write_batch_timer = Stopwatch::start_new();
        store
            .write_fts_content_batch(batch)
            .await
            .map_err(ExtractError::storage)?;
        benchmark.insert_duration_ms("fts_tantivy.write_batch", write_batch_timer.elapsed());
        benchmark.insert_label("fts_tantivy.write_mode", "fts_content_write_batch");
        benchmark.insert_count("fts_tantivy.files_hashed", summary.files_hashed);
        benchmark.insert_count(
            "fts_tantivy.files_hash_unchanged",
            summary.files_hash_unchanged,
        );
        benchmark.insert_count("fts_tantivy.files_changed", summary.files_changed);
        benchmark.insert_count("fts_tantivy.indexed_files", summary.indexed_files);
        benchmark.insert_count("fts_tantivy.skipped_files", summary.skipped_files);
        benchmark.insert_count(
            "fts_tantivy.skipped_binary_or_unreadable",
            summary.skipped_binary_or_unreadable,
        );
        benchmark.insert_count("fts_tantivy.indexed_bytes", indexed_bytes);
        benchmark.insert_count("fts_tantivy.fingerprint_entries", fingerprint_entries.len());

        let route_content_hash_timer = Stopwatch::start_new();
        let route_content_hash = route_content_hash(fingerprint_entries);
        benchmark.insert_duration_ms(
            "fts_tantivy.route_content_hash",
            route_content_hash_timer.elapsed(),
        );
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
                route: FTS_TANTIVY_ROUTE,
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
        benchmark.insert_duration_ms("fts_tantivy.route_complete", route_complete_timer.elapsed());

        let close_stale_timer = Stopwatch::start_new();
        summary.stale_fts_documents_closed = store
            .close_stale_fts_documents_for_workspace(CloseStaleFtsDocumentsInput {
                workspace_id: started_run.workspace_id,
                run_id: started_run.run_id,
                provider: FTS_PROVIDER,
                route: FTS_TANTIVY_ROUTE,
                scope: FTS_SCOPE,
                scope_key: &started_run.workspace_root_uri,
            })
            .await
            .map_err(ExtractError::storage)?;
        benchmark.insert_duration_ms(
            "fts_tantivy.close_stale_documents",
            close_stale_timer.elapsed(),
        );
        benchmark.insert_label(
            "fts_tantivy.stale_fts_documents_closed",
            summary.stale_fts_documents_closed.to_string(),
        );

        let finish_run_timer = Stopwatch::start_new();
        store
            .finish_run(started_run.run_id, "complete")
            .await
            .map_err(ExtractError::storage)?;
        benchmark.insert_duration_ms("fts_tantivy.finish_run", finish_run_timer.elapsed());

        summary.benchmark = benchmark;

        Ok(summary)
    }
}

fn runtime_artifact_exclusions(
    workspace_root: &Path,
    db_path: &Path,
    index_path: &Path,
) -> (Vec<String>, Vec<String>) {
    let mut directories = Vec::new();
    let mut files = Vec::new();

    if let Some(index_relative_path) = workspace_relative_artifact_path(workspace_root, index_path)
    {
        directories.push(index_relative_path);
    }
    if let Some(db_relative_path) = workspace_relative_artifact_path(workspace_root, db_path) {
        files.push(db_relative_path.clone());
        files.push(format!("{db_relative_path}-journal"));
        files.push(format!("{db_relative_path}-shm"));
        files.push(format!("{db_relative_path}-wal"));
    }

    (directories, files)
}

fn workspace_relative_artifact_path(workspace_root: &Path, artifact_path: &Path) -> Option<String> {
    if !artifact_path.is_absolute()
        && artifact_path
            .components()
            .any(|component| component == Component::ParentDir)
    {
        return None;
    }

    let absolute_path = if artifact_path.is_absolute() {
        artifact_path.to_path_buf()
    } else {
        workspace_root.join(artifact_path)
    };
    absolute_path
        .strip_prefix(workspace_root)
        .ok()
        .map(crate::fts::normalize_relative_path)
        .filter(|relative_path| !relative_path.is_empty())
}
