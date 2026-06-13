use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::{ExtractError, Result};
use crate::model::DocumentSymbolRequest;

pub fn validate_document_symbol_request(
    request: DocumentSymbolRequest,
) -> Result<DocumentSymbolRequest> {
    let workspace_root = canonicalize_path(&request.workspace_root, "canonicalize workspace root")?;
    let package_path = canonicalize_path(&request.package_path, "canonicalize package path")?;
    let file_path = canonicalize_path(&request.file_path, "canonicalize source file")?;

    if !file_path.starts_with(&workspace_root) {
        return Err(ExtractError::InvalidPath {
            path: file_path,
            workspace_root,
            message: "source file is outside the workspace root".to_string(),
        });
    }

    Ok(DocumentSymbolRequest {
        workspace_root,
        package_path,
        file_path,
    })
}

pub fn workspace_relative_path(workspace_root: &Path, file_path: &Path) -> Result<String> {
    let relative =
        file_path
            .strip_prefix(workspace_root)
            .map_err(|_| ExtractError::InvalidPath {
                path: file_path.to_path_buf(),
                workspace_root: workspace_root.to_path_buf(),
                message: "source file is outside the workspace root".to_string(),
            })?;

    path_with_forward_slashes(relative, workspace_root, file_path)
}

pub fn file_uri(path: &Path) -> Result<String> {
    if !path.is_absolute() {
        return Err(ExtractError::InvalidPath {
            path: path.to_path_buf(),
            workspace_root: PathBuf::new(),
            message: "file URI requires an absolute path".to_string(),
        });
    }

    let value = path.to_str().ok_or_else(|| ExtractError::InvalidPath {
        path: path.to_path_buf(),
        workspace_root: PathBuf::new(),
        message: "file path is not valid UTF-8".to_string(),
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

fn canonicalize_path(path: &Path, context: &str) -> Result<PathBuf> {
    path.canonicalize()
        .map_err(|source| ExtractError::io(context, Some(path.to_path_buf()), source))
}

fn path_with_forward_slashes(
    relative: &Path,
    workspace_root: &Path,
    file_path: &Path,
) -> Result<String> {
    let mut parts = Vec::new();

    for component in relative.components() {
        match component {
            Component::Normal(value) => {
                let part = value.to_str().ok_or_else(|| ExtractError::InvalidPath {
                    path: file_path.to_path_buf(),
                    workspace_root: workspace_root.to_path_buf(),
                    message: "relative source path is not valid UTF-8".to_string(),
                })?;
                parts.push(part.to_string());
            }
            Component::CurDir => {}
            _ => {
                return Err(ExtractError::InvalidPath {
                    path: file_path.to_path_buf(),
                    workspace_root: workspace_root.to_path_buf(),
                    message: "relative source path contains an unsupported component".to_string(),
                });
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

#[cfg(test)]
mod tests {
    use std::env;
    use std::error::Error;
    use std::io;

    use super::*;

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
        let crates_dir = manifest_dir.parent().ok_or_else(|| {
            io::Error::other("extract crate manifest dir has no parent directory")
        })?;
        let repo_root = crates_dir
            .parent()
            .ok_or_else(|| io::Error::other("crates directory has no parent directory"))?;

        Ok(repo_root.to_path_buf())
    }
}
