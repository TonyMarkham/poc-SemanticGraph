use crate::{
    ExtractError, ExtractResult,
    document_symbols::paths::file_uri,
    fts::{
        FTS_PROVIDER, FTS_ROUTE, FTS_SCOPE, FtsExclusionSet, FtsExtractionOptions,
        FtsExtractionSummary, FtsFileDiscovery, FtsSkipReason,
    },
};

use semantic_graph_config::FtsConfig;
use semantic_graph_db_manager::{
    CloseStaleFtsDocumentsInput, FileInput, FtsDocumentInput, RouteStatusCompleteInput,
    RouteStatusFailInput, RouteStatusStartInput, WriteHandle,
};
use serde_json::json;
use sha2::Digest;
use std::{fs, path::Path};

const MAX_INDEXED_FILE_BYTES: u64 = 10 * 1024 * 1024;

pub struct FtsExtractionRunner;

impl FtsExtractionRunner {
    pub async fn run(
        store: &WriteHandle,
        workspace_root: &Path,
        config: &FtsConfig,
        options: FtsExtractionOptions,
    ) -> ExtractResult<FtsExtractionSummary> {
        let workspace_root = workspace_root
            .canonicalize()
            .map_err(|source| ExtractError::io("canonicalize FTS workspace root", None, source))?;
        let workspace_root_uri = file_uri(&workspace_root)?;
        let workspace_id = store
            .create_workspace(&workspace_root_uri, "mixed")
            .await
            .map_err(ExtractError::storage)?;
        let run_id = store
            .start_run(workspace_id, FTS_PROVIDER, None, None)
            .await
            .map_err(ExtractError::storage)?;

        let result = Self::run_started(
            store,
            &workspace_root,
            &workspace_root_uri,
            workspace_id,
            run_id,
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

        result
    }

    async fn run_started(
        store: &WriteHandle,
        workspace_root: &Path,
        workspace_root_uri: &str,
        workspace_id: i64,
        run_id: i64,
        config: &FtsConfig,
        options: FtsExtractionOptions,
    ) -> ExtractResult<FtsExtractionSummary> {
        store
            .start_route_status(RouteStatusStartInput {
                workspace_id,
                route: FTS_ROUTE,
                scope: FTS_SCOPE,
                scope_key: workspace_root_uri,
                file_id: None,
                provider: FTS_PROVIDER,
                provider_version: None,
                content_hash: None,
                run_id,
                diagnostics_json: json!({ "status": "running" }),
            })
            .await
            .map_err(ExtractError::storage)?;

        let exclusions = FtsExclusionSet::new(workspace_root, config, options)?;
        let discovery = FtsFileDiscovery::discover(workspace_root, &exclusions)?;
        let mut summary = FtsExtractionSummary {
            workspace_id,
            run_id,
            scanned_files: discovery.scanned_files(),
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
        let mut fingerprint_entries = Vec::new();

        for file in discovery.files() {
            let Some(text) = read_indexable_text(file.absolute_path(), &mut summary)? else {
                continue;
            };
            let content_hash = sha256_hex(text.as_bytes());
            let file_uri = file_uri(file.absolute_path())?;
            let language = file.language().as_str();
            let file_id = store
                .upsert_file(FileInput {
                    workspace_id,
                    uri: &file_uri,
                    path: file.relative_path(),
                    language,
                    content_hash: Some(&content_hash),
                    last_seen_run_id: Some(run_id),
                    properties_json: json!({ "route": FTS_ROUTE }),
                })
                .await
                .map_err(ExtractError::storage)?;
            store
                .upsert_fts_document(FtsDocumentInput {
                    workspace_id,
                    file_id,
                    path: file.relative_path(),
                    language,
                    content_hash: &content_hash,
                    byte_len: text.len() as i64,
                    run_id,
                    content: &text,
                    properties_json: json!({ "route": FTS_ROUTE }),
                })
                .await
                .map_err(ExtractError::storage)?;
            summary.count_indexed_file();
            fingerprint_entries.push(format!("{}:{content_hash}", file.relative_path()));
        }

        let route_content_hash = route_content_hash(fingerprint_entries);
        let diagnostics_json = json!({
            "scanned_files": summary.scanned_files,
            "indexed_files": summary.indexed_files,
            "skipped_files": summary.skipped_files,
            "skipped_directories": summary.skipped_directories,
            "skipped_by_config": summary.skipped_by_config,
            "skipped_by_no_rust": summary.skipped_by_no_rust,
            "skipped_by_no_csharp": summary.skipped_by_no_csharp,
            "skipped_by_no_submodules": summary.skipped_by_no_submodules,
            "skipped_binary_or_unreadable": summary.skipped_binary_or_unreadable,
        });
        store
            .complete_route_status(RouteStatusCompleteInput {
                workspace_id,
                route: FTS_ROUTE,
                scope: FTS_SCOPE,
                scope_key: workspace_root_uri,
                provider: FTS_PROVIDER,
                provider_version: None,
                content_hash: Some(&route_content_hash),
                run_id,
                diagnostics_json,
            })
            .await
            .map_err(ExtractError::storage)?;

        summary.stale_fts_documents_closed = store
            .close_stale_fts_documents_for_workspace(CloseStaleFtsDocumentsInput {
                workspace_id,
                run_id,
                provider: FTS_PROVIDER,
                route: FTS_ROUTE,
                scope: FTS_SCOPE,
                scope_key: workspace_root_uri,
            })
            .await
            .map_err(ExtractError::storage)?;
        store
            .finish_run(run_id, "complete")
            .await
            .map_err(ExtractError::storage)?;

        Ok(summary)
    }
}

fn read_indexable_text(
    path: &Path,
    summary: &mut FtsExtractionSummary,
) -> ExtractResult<Option<String>> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(_error) => {
            summary.count_runtime_skip(FtsSkipReason::BinaryOrUnreadable);
            return Ok(None);
        }
    };
    if metadata.len() > MAX_INDEXED_FILE_BYTES {
        summary.count_runtime_skip(FtsSkipReason::BinaryOrUnreadable);
        return Ok(None);
    }

    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(_error) => {
            summary.count_runtime_skip(FtsSkipReason::BinaryOrUnreadable);
            return Ok(None);
        }
    };
    if bytes.contains(&0) {
        summary.count_runtime_skip(FtsSkipReason::BinaryOrUnreadable);
        return Ok(None);
    }

    match String::from_utf8(bytes) {
        Ok(text) => Ok(Some(text)),
        Err(_error) => {
            summary.count_runtime_skip(FtsSkipReason::BinaryOrUnreadable);
            Ok(None)
        }
    }
}

fn route_content_hash(mut entries: Vec<String>) -> String {
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
