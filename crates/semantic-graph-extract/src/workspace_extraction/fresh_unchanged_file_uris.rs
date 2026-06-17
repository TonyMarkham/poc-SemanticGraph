use crate::{
    ExtractError, ExtractResult, model::RouteName, providers::rust_analyzer::RustAnalyzerProvider,
    workspace_extraction::WorkspaceFileHash,
};

use semantic_graph_db_manager::WriteHandle;
use std::collections::HashSet;

pub(crate) async fn fresh_unchanged_file_uris(
    store: &WriteHandle,
    workspace_id: i64,
    provider: &RustAnalyzerProvider,
    file_hashes: &[WorkspaceFileHash],
) -> ExtractResult<HashSet<String>> {
    let stored_hashes = store
        .file_route_content_hashes(
            workspace_id,
            RouteName::RUST_DOCUMENT_SYMBOLS.as_str(),
            provider.provider_id().as_str(),
        )
        .await
        .map_err(ExtractError::storage)?;
    let unchanged = file_hashes
        .iter()
        .filter(|file_hash| {
            stored_hashes.get(&file_hash.uri).and_then(Option::as_deref)
                == Some(file_hash.content_hash.as_str())
        })
        .map(|file_hash| file_hash.uri.clone())
        .collect::<HashSet<_>>();

    Ok(unchanged)
}
