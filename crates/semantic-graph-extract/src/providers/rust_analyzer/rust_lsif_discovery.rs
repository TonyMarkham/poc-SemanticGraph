use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;
use serde_json::Value;

use crate::document_symbols::paths::validate_document_symbol_batch_request;
use crate::error::{ExtractError, Result};
use crate::model::DocumentSymbolBatchRequest;

#[derive(Debug, Deserialize)]
struct LsifDocumentVertex {
    #[serde(rename = "type")]
    vertex_type: String,
    label: String,
    uri: String,
    #[serde(rename = "languageId")]
    language_id: Option<String>,
}

pub fn discover_rust_source_files_with_lsif(
    binary: &str,
    workspace_root: &Path,
    package_path: &Path,
) -> Result<Vec<PathBuf>> {
    let request = validate_document_symbol_batch_request(DocumentSymbolBatchRequest {
        workspace_root: workspace_root.to_path_buf(),
        package_path: package_path.to_path_buf(),
        file_paths: Vec::new(),
    })?;
    let output = Command::new(binary)
        .arg("lsif")
        .arg(&request.package_path)
        .output()
        .map_err(|source| {
            ExtractError::io(
                format!("run {binary} lsif for Rust source discovery"),
                Some(request.package_path.clone()),
                source,
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ExtractError::process(
            "rust-analyzer",
            binary,
            format!("lsif exited with status {}: {stderr}", output.status),
        ));
    }

    let stdout = String::from_utf8(output.stdout).map_err(|source| {
        ExtractError::io(
            "decode rust-analyzer lsif stdout as UTF-8",
            Some(request.package_path.clone()),
            std::io::Error::new(std::io::ErrorKind::InvalidData, source),
        )
    })?;

    discover_rust_source_files_from_lsif(&request.workspace_root, &request.package_path, &stdout)
}

pub fn discover_rust_source_files_from_lsif(
    workspace_root: &Path,
    package_path: &Path,
    lsif_output: &str,
) -> Result<Vec<PathBuf>> {
    let request = validate_document_symbol_batch_request(DocumentSymbolBatchRequest {
        workspace_root: workspace_root.to_path_buf(),
        package_path: package_path.to_path_buf(),
        file_paths: Vec::new(),
    })?;

    let mut file_paths = Vec::new();

    for line in lsif_output.lines().filter(|line| !line.trim().is_empty()) {
        let value: Value = serde_json::from_str(line)
            .map_err(|source| ExtractError::json("parse LSIF line", source))?;

        if value.get("label").and_then(Value::as_str) != Some("document") {
            continue;
        }

        let document: LsifDocumentVertex = serde_json::from_value(value)
            .map_err(|source| ExtractError::json("parse LSIF document vertex", source))?;
        if document.vertex_type != "vertex"
            || document.label != "document"
            || document.language_id.as_deref() != Some("rust")
        {
            continue;
        }

        let path = file_uri_to_path(&document.uri)?;
        if path.starts_with(&request.package_path) {
            file_paths.push(path);
        }
    }

    file_paths.sort();
    file_paths.dedup();

    if file_paths.is_empty() {
        return Err(ExtractError::response_shape(
            "rust-analyzer",
            "rust-analyzer lsif",
            "LSIF output contained no Rust document vertices under the package path",
        ));
    }

    let validated = validate_document_symbol_batch_request(DocumentSymbolBatchRequest {
        workspace_root: request.workspace_root,
        package_path: request.package_path,
        file_paths,
    })?;

    Ok(validated.file_paths)
}

fn file_uri_to_path(uri: &str) -> Result<PathBuf> {
    let path = uri
        .strip_prefix("file://")
        .ok_or_else(|| ExtractError::InvalidPath {
            path: PathBuf::from(uri),
            workspace_root: PathBuf::new(),
            message: "LSIF document URI is not a file URI".to_string(),
        })?;

    if !path.starts_with('/') {
        return Err(ExtractError::InvalidPath {
            path: PathBuf::from(uri),
            workspace_root: PathBuf::new(),
            message: "LSIF document URI with a host is not supported".to_string(),
        });
    }

    percent_decode_path(path)
}

fn percent_decode_path(value: &str) -> Result<PathBuf> {
    let mut bytes = Vec::with_capacity(value.len());
    let mut index = 0;
    let source = value.as_bytes();

    while index < source.len() {
        if source[index] == b'%' {
            if index + 2 >= source.len() {
                return Err(ExtractError::InvalidPath {
                    path: PathBuf::from(value),
                    workspace_root: PathBuf::new(),
                    message: "LSIF document URI has incomplete percent encoding".to_string(),
                });
            }
            let high = hex_value(source[index + 1]).ok_or_else(|| ExtractError::InvalidPath {
                path: PathBuf::from(value),
                workspace_root: PathBuf::new(),
                message: "LSIF document URI has invalid percent encoding".to_string(),
            })?;
            let low = hex_value(source[index + 2]).ok_or_else(|| ExtractError::InvalidPath {
                path: PathBuf::from(value),
                workspace_root: PathBuf::new(),
                message: "LSIF document URI has invalid percent encoding".to_string(),
            })?;
            bytes.push((high << 4) | low);
            index += 3;
        } else {
            bytes.push(source[index]);
            index += 1;
        }
    }

    let decoded = String::from_utf8(bytes).map_err(|source| {
        ExtractError::io(
            "decode LSIF document URI path as UTF-8",
            Some(PathBuf::from(value)),
            std::io::Error::new(std::io::ErrorKind::InvalidData, source),
        )
    })?;

    Ok(PathBuf::from(decoded))
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn filters_lsif_documents_to_package_path_deterministically()
    -> std::result::Result<(), Box<dyn Error>> {
        let workspace_root = temp_workspace_path("lsif")?;
        let package_path = workspace_root.join("crates/wip");
        fs::create_dir_all(package_path.join("src"))?;
        fs::create_dir_all(workspace_root.join("crates/other/src"))?;
        fs::write(package_path.join("src/lib.rs"), "")?;
        fs::write(package_path.join("src/models.rs"), "")?;
        fs::write(package_path.join("src/pipeline.rs"), "")?;
        fs::write(workspace_root.join("crates/other/src/lib.rs"), "")?;

        let root_uri = workspace_root.to_string_lossy();
        let lsif_output = format!(
            r#"{{"id":0,"type":"vertex","label":"metaData","projectRoot":"file://{root_uri}/crates/wip"}}"#
        ) + "\n"
            + &format!(
                r#"{{"id":1,"type":"vertex","label":"document","uri":"file://{root_uri}/crates/wip/src/pipeline.rs","languageId":"rust"}}"#
            )
            + "\n"
            + &format!(
                r#"{{"id":2,"type":"vertex","label":"document","uri":"file://{root_uri}/crates/other/src/lib.rs","languageId":"rust"}}"#
            )
            + "\n"
            + &format!(
                r#"{{"id":3,"type":"vertex","label":"document","uri":"file://{root_uri}/crates/wip/src/lib.rs","languageId":"rust"}}"#
            )
            + "\n"
            + &format!(
                r#"{{"id":4,"type":"vertex","label":"document","uri":"file://{root_uri}/crates/wip/src/models.rs","languageId":"rust"}}"#
            )
            + "\n";

        let files =
            discover_rust_source_files_from_lsif(&workspace_root, &package_path, &lsif_output)?;

        assert_eq!(
            files,
            vec![
                package_path.join("src/lib.rs").canonicalize()?,
                package_path.join("src/models.rs").canonicalize()?,
                package_path.join("src/pipeline.rs").canonicalize()?,
            ]
        );
        Ok(())
    }

    #[test]
    fn decodes_percent_encoded_file_uris() -> std::result::Result<(), Box<dyn Error>> {
        assert_eq!(
            file_uri_to_path("file:///tmp/a%20path/lib.rs")?,
            PathBuf::from("/tmp/a path/lib.rs")
        );
        Ok(())
    }

    fn temp_workspace_path(name: &str) -> std::result::Result<PathBuf, Box<dyn Error>> {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        Ok(std::env::temp_dir().join(format!(
            "poc-semanticgraph-extract-{name}-{}-{stamp}",
            std::process::id()
        )))
    }
}
