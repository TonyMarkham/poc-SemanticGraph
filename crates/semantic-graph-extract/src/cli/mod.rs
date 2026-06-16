#![allow(clippy::module_inception)]

mod cli;
mod command;
mod rust_file_extractions;
mod rust_file_mode;

// ---------------------------------------------------------------------------------------------- //

pub use cli::Cli;
pub use command::Command;
pub use rust_file_extractions::RustFileExtractions;
pub use rust_file_mode::RustFileMode;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    ExtractError, ExtractResult,
    document_symbols::paths::{file_uri, workspace_relative_path},
    workspace_extraction::WorkspaceExtractionRoutes,
};

use semantic_graph_config::{LoadOptions, resolve_database_path};

use std::path::{Component, Path, PathBuf};

pub fn resolve_cli_database_path(
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

pub fn resolve_rust_file_mode(
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

pub fn resolve_rust_workspace_routes(
    calls: bool,
    references: bool,
    symbols: bool,
) -> WorkspaceExtractionRoutes {
    WorkspaceExtractionRoutes::from_selectors(symbols, references, calls)
}

pub fn validate_deleted_rust_file_request(
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

pub fn resolve_deleted_file_path(
    workspace_root: &Path,
    file_path: &Path,
) -> ExtractResult<PathBuf> {
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

pub fn normalize_lexical_path(path: &Path) -> PathBuf {
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

pub fn symbol_key_belongs_to_file(symbol_key: &str, file_scope_key: &str) -> bool {
    let file_node_key = format!("file:{file_scope_key}");
    symbol_key == file_node_key
        || symbol_key
            .strip_prefix(file_scope_key)
            .is_some_and(|suffix| suffix.starts_with('#'))
}
