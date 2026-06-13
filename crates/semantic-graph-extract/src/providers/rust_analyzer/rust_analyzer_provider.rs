use std::fs;
use std::process::Command;

use lsp_types::DocumentSymbolResponse;
use serde_json::{Value, json};

use crate::document_symbols::paths::{file_uri, validate_document_symbol_request};
use crate::error::{ExtractError, Result};
use crate::lsp_stdio::LspStdioClient;
use crate::model::{DocumentSymbolExtraction, DocumentSymbolRequest, GraphLanguage, ProviderId};
use crate::provider::DocumentSymbolProvider;
use crate::providers::rust_analyzer::RustDocumentSymbolMapper;

const DEFAULT_TIMEOUT_MS: u64 = 30_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustAnalyzerProvider {
    binary: String,
    timeout_ms: u64,
}

impl RustAnalyzerProvider {
    pub fn new() -> Self {
        Self {
            binary: "rust-analyzer".to_string(),
            timeout_ms: DEFAULT_TIMEOUT_MS,
        }
    }

    pub fn with_binary(binary: impl Into<String>) -> Self {
        Self {
            binary: binary.into(),
            timeout_ms: DEFAULT_TIMEOUT_MS,
        }
    }

    async fn run(&self, request: DocumentSymbolRequest) -> Result<DocumentSymbolExtraction> {
        let request = validate_document_symbol_request(request)?;
        let workspace_uri = file_uri(&request.workspace_root)?;
        let source_uri = file_uri(&request.file_path)?;
        let source_text = fs::read_to_string(&request.file_path).map_err(|source| {
            ExtractError::io(
                "read source file for textDocument/didOpen",
                Some(request.file_path.clone()),
                source,
            )
        })?;
        let fallback_version = discover_binary_version(&self.binary);
        let mut client = LspStdioClient::spawn(&self.binary, self.provider_id())?;

        let extraction_result = async {
            let initialize_result = client
                .request(
                    "initialize",
                    initialize_params(&workspace_uri),
                    self.timeout_ms,
                )
                .await?;
            let provider_version =
                server_info_version(&initialize_result).or_else(|| fallback_version.clone());

            client
                .notify("initialized", Some(json!({})), self.timeout_ms)
                .await?;
            client
                .notify(
                    "textDocument/didOpen",
                    Some(json!({
                        "textDocument": {
                            "uri": source_uri,
                            "languageId": "rust",
                            "version": 1,
                            "text": source_text
                        }
                    })),
                    self.timeout_ms,
                )
                .await?;
            let document_symbol_result = client
                .request(
                    "textDocument/documentSymbol",
                    json!({
                        "textDocument": {
                            "uri": source_uri
                        }
                    }),
                    self.timeout_ms,
                )
                .await?;
            let response: Option<DocumentSymbolResponse> =
                serde_json::from_value(document_symbol_result).map_err(|source| {
                    ExtractError::json("parse documentSymbol response", source)
                })?;
            let response = response.ok_or_else(|| {
                ExtractError::response_shape(
                    self.provider_id().as_str(),
                    "textDocument/documentSymbol",
                    "rust-analyzer returned null for document symbols",
                )
            })?;

            RustDocumentSymbolMapper::map_response(
                request,
                response,
                provider_version,
                json!({
                    "initialize": initialize_result
                }),
            )
        }
        .await;

        let shutdown_result = client.shutdown(self.timeout_ms).await;

        match extraction_result {
            Ok(extraction) => {
                shutdown_result?;
                Ok(extraction)
            }
            Err(error) => {
                let _cleanup_result = shutdown_result;
                Err(error)
            }
        }
    }
}

impl Default for RustAnalyzerProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentSymbolProvider for RustAnalyzerProvider {
    fn provider_id(&self) -> ProviderId {
        ProviderId::rust_analyzer()
    }

    fn language(&self) -> GraphLanguage {
        GraphLanguage::Rust
    }

    async fn extract_document_symbols(
        &self,
        request: DocumentSymbolRequest,
    ) -> Result<DocumentSymbolExtraction> {
        self.run(request).await
    }
}

fn initialize_params(workspace_uri: &str) -> Value {
    json!({
        "processId": std::process::id(),
        "rootUri": workspace_uri,
        "capabilities": {
            "textDocument": {
                "documentSymbol": {
                    "hierarchicalDocumentSymbolSupport": true,
                    "symbolKind": {
                        "valueSet": [
                            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13,
                            14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
                            25, 26
                        ]
                    }
                }
            },
            "workspace": {
                "workspaceFolders": true
            }
        },
        "workspaceFolders": [
            {
                "uri": workspace_uri,
                "name": "workspace"
            }
        ]
    })
}

fn server_info_version(initialize_result: &Value) -> Option<String> {
    initialize_result
        .get("serverInfo")
        .and_then(|server_info| server_info.get("version"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn discover_binary_version(binary: &str) -> Option<String> {
    let output = Command::new(binary).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8(output.stdout).ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
