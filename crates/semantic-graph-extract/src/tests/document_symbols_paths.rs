use std::env;
use std::error::Error;
use std::io;
use std::path::{Path, PathBuf};

use crate::document_symbols::paths::{file_uri, validate_document_symbol_request};
use crate::error::ExtractError;
use crate::model::DocumentSymbolRequest;

#[test]
fn rejects_file_outside_workspace_root() -> std::result::Result<(), Box<dyn Error>> {
    let repo_root = repo_root()?;
    let workspace_root = repo_root.join("crates/wip");
    let outside_file = repo_root.join("Cargo.toml");

    let result = validate_document_symbol_request(DocumentSymbolRequest {
        workspace_root,
        package_path: repo_root.join("crates/wip"),
        file_path: outside_file,
    });

    assert!(matches!(result, Err(ExtractError::InvalidPath { .. })));
    Ok(())
}

#[test]
fn file_uri_percent_encodes_spaces() -> std::result::Result<(), Box<dyn Error>> {
    let uri = file_uri(Path::new("/tmp/a path/lib.rs"))?;
    assert_eq!(uri, "file:///tmp/a%20path/lib.rs");
    Ok(())
}

fn repo_root() -> std::result::Result<PathBuf, Box<dyn Error>> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let crates_dir = manifest_dir
        .parent()
        .ok_or_else(|| io::Error::other("extract crate manifest dir has no parent directory"))?;
    let repo_root = crates_dir
        .parent()
        .ok_or_else(|| io::Error::other("crates directory has no parent directory"))?;

    Ok(repo_root.to_path_buf())
}
