use crate::{
    ExtractError, ExtractResult, model::ProviderId, workspace_extraction::WorkspaceFileHash,
};

use semantic_graph_db_manager::WriteHandle;
use std::collections::HashSet;

pub async fn fresh_unchanged_file_uris(
    store: &WriteHandle,
    workspace_id: i64,
    route: &str,
    provider_id: ProviderId,
    file_hashes: &[WorkspaceFileHash],
) -> ExtractResult<HashSet<String>> {
    let stored_hashes = store
        .file_route_content_hashes(workspace_id, route, provider_id.as_str())
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
