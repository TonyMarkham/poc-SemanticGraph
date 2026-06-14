use crate::{
    ExtractError, ExtractResult,
    model::{DocumentSymbolBatchRequest, DocumentSymbolRequest},
};

use sha2::{Digest, Sha256};
use std::path::{Component, Path, PathBuf};

pub fn validate_document_symbol_request(
    request: DocumentSymbolRequest,
) -> ExtractResult<DocumentSymbolRequest> {
    let workspace_root = canonicalize_path(&request.workspace_root, "canonicalize workspace root")?;
    let package_path = canonicalize_path(&request.package_path, "canonicalize package path")?;
    let file_path = canonicalize_path(&request.file_path, "canonicalize source file")?;

    validate_package_path(&workspace_root, &package_path)?;
    validate_source_path(&workspace_root, &package_path, &file_path)?;

    Ok(DocumentSymbolRequest {
        workspace_root,
        package_path,
        file_path,
    })
}

pub fn validate_document_symbol_batch_request(
    request: DocumentSymbolBatchRequest,
) -> ExtractResult<DocumentSymbolBatchRequest> {
    let workspace_root = canonicalize_path(&request.workspace_root, "canonicalize workspace root")?;
    let package_path = canonicalize_path(&request.package_path, "canonicalize package path")?;
    validate_package_path(&workspace_root, &package_path)?;

    let mut file_paths = Vec::with_capacity(request.file_paths.len());
    for file_path in request.file_paths {
        let file_path = canonicalize_path(&file_path, "canonicalize source file")?;
        validate_source_path(&workspace_root, &package_path, &file_path)?;
        file_paths.push(file_path);
    }
    file_paths.sort();
    file_paths.dedup();

    Ok(DocumentSymbolBatchRequest {
        workspace_root,
        package_path,
        file_paths,
    })
}

pub fn workspace_relative_path(workspace_root: &Path, file_path: &Path) -> ExtractResult<String> {
    let relative = file_path.strip_prefix(workspace_root).map_err(|_| {
        ExtractError::invalid_path(
            file_path,
            workspace_root,
            "source file is outside the workspace root",
        )
    })?;

    path_with_forward_slashes(relative, workspace_root, file_path)
}

pub fn file_uri(path: &Path) -> ExtractResult<String> {
    if !path.is_absolute() {
        return Err(ExtractError::invalid_path(
            path,
            PathBuf::new(),
            "file URI requires an absolute path",
        ));
    }

    let value = path.to_str().ok_or_else(|| {
        ExtractError::invalid_path(path, PathBuf::new(), "file path is not valid UTF-8")
    })?;

    Ok(format!("file://{}", percent_encode_path(value)))
}

pub fn file_symbol_key(file_uri: &str) -> String {
    format!("file:{file_uri}")
}

pub fn content_hash(contents: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(contents.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn basename_from_relative_path(relative_path: &str) -> String {
    Path::new(relative_path)
        .file_name()
        .and_then(|value| value.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| relative_path.to_string())
}

pub fn percent_encode_component(value: &str) -> String {
    percent_encode_path(value)
}

fn canonicalize_path(path: &Path, context: &str) -> ExtractResult<PathBuf> {
    path.canonicalize()
        .map_err(|source| ExtractError::io(context, Some(path.to_path_buf()), source))
}

fn validate_package_path(workspace_root: &Path, package_path: &Path) -> ExtractResult<()> {
    if !package_path.starts_with(workspace_root) {
        return Err(ExtractError::invalid_path(
            package_path,
            workspace_root,
            "package path is outside the workspace root",
        ));
    }

    Ok(())
}

fn validate_source_path(
    workspace_root: &Path,
    package_path: &Path,
    file_path: &Path,
) -> ExtractResult<()> {
    if !file_path.starts_with(workspace_root) {
        return Err(ExtractError::invalid_path(
            file_path,
            workspace_root,
            "source file is outside the workspace root",
        ));
    }

    if !file_path.starts_with(package_path) {
        return Err(ExtractError::invalid_path(
            file_path,
            workspace_root,
            "source file is outside the package path",
        ));
    }

    Ok(())
}

fn path_with_forward_slashes(
    relative: &Path,
    workspace_root: &Path,
    file_path: &Path,
) -> ExtractResult<String> {
    let mut parts = Vec::new();

    for component in relative.components() {
        match component {
            Component::Normal(value) => {
                let part = value.to_str().ok_or_else(|| {
                    ExtractError::invalid_path(
                        file_path,
                        workspace_root,
                        "relative source path is not valid UTF-8",
                    )
                })?;
                parts.push(part.to_string());
            }
            Component::CurDir => {}
            _ => {
                return Err(ExtractError::invalid_path(
                    file_path,
                    workspace_root,
                    "relative source path contains an unsupported component",
                ));
            }
        }
    }

    Ok(parts.join("/"))
}

fn percent_encode_path(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut encoded = String::new();

    for byte in value.as_bytes() {
        let is_unreserved =
            byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'.' | b'_' | b'~' | b'/');

        if is_unreserved {
            encoded.push(char::from(*byte));
        } else {
            encoded.push('%');
            encoded.push(char::from(HEX[(byte >> 4) as usize]));
            encoded.push(char::from(HEX[(byte & 0x0F) as usize]));
        }
    }

    encoded
}
