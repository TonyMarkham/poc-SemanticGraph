use crate::{
    ExtractError, ExtractResult,
    document_symbols::paths::{content_hash, file_uri},
    model::DocumentSymbolBatchRequest,
    workspace_extraction::WorkspaceFileHash,
};

use std::fs;

pub(crate) fn workspace_file_hashes(
    document_request: &DocumentSymbolBatchRequest,
) -> ExtractResult<Vec<WorkspaceFileHash>> {
    let mut hashes = Vec::with_capacity(document_request.file_paths.len());
    for file_path in &document_request.file_paths {
        let file_contents = fs::read_to_string(file_path).map_err(|source| {
            ExtractError::io(
                "read source file for workspace hash",
                Some(file_path.clone()),
                source,
            )
        })?;
        hashes.push(WorkspaceFileHash {
            file_path: file_path.clone(),
            uri: file_uri(file_path)?,
            content_hash: content_hash(&file_contents),
        });
    }

    Ok(hashes)
}
