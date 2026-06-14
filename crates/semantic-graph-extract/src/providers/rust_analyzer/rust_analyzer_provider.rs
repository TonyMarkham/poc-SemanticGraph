use crate::{
    ExtractError, ExtractResult,
    document_symbols::paths::{
        validate_document_symbol_batch_request, validate_document_symbol_request,
    },
    model::{
        DocumentSymbolBatchExtraction, DocumentSymbolBatchRequest, DocumentSymbolExtraction,
        DocumentSymbolRequest, GraphLanguage, ProviderId,
    },
    provider::DocumentSymbolProvider,
    providers::rust_analyzer::RustDocumentSymbolMapper,
};

use lsp_types::DocumentSymbolResponse;
use serde_json::json;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustAnalyzerProvider;

impl RustAnalyzerProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn with_binary(_binary: impl Into<String>) -> Self {
        Self
    }

    pub async fn extract_document_symbol_batch(
        &self,
        request: DocumentSymbolBatchRequest,
    ) -> ExtractResult<DocumentSymbolBatchExtraction> {
        self.run_batch(request).await
    }

    pub fn discover_rust_source_files(
        &self,
        workspace_root: &Path,
        package_path: &Path,
    ) -> ExtractResult<Vec<PathBuf>> {
        let request = validate_document_symbol_batch_request(DocumentSymbolBatchRequest {
            workspace_root: workspace_root.to_path_buf(),
            package_path: package_path.to_path_buf(),
            file_paths: Vec::new(),
        })?;
        let model = rust_analyzer_lib::load_package(&request.workspace_root, &request.package_path)
            .map_err(|source| facade_error("rust-analyzer-lib load_package", source))?;
        let files = rust_analyzer_lib::package_source_files(&model, &request.package_path);
        if files.is_empty() {
            return Err(ExtractError::response_shape(
                self.provider_id().as_str(),
                "rust-analyzer-lib package_source_files",
                "rust-analyzer-lib returned no Rust source files under the package path",
            ));
        }

        Ok(files)
    }

    pub fn discover_rust_workspace_source_files(
        &self,
        workspace_root: &Path,
    ) -> ExtractResult<Vec<PathBuf>> {
        let request = validate_document_symbol_batch_request(DocumentSymbolBatchRequest {
            workspace_root: workspace_root.to_path_buf(),
            package_path: workspace_root.to_path_buf(),
            file_paths: Vec::new(),
        })?;
        let model = rust_analyzer_lib::load_workspace(&request.workspace_root)
            .map_err(|source| facade_error("rust-analyzer-lib load_workspace", source))?;
        let files = rust_analyzer_lib::workspace_source_files(&model);
        if files.is_empty() {
            return Err(ExtractError::response_shape(
                self.provider_id().as_str(),
                "rust-analyzer-lib workspace_source_files",
                "rust-analyzer-lib returned no Rust workspace source files",
            ));
        }

        Ok(files)
    }

    async fn run(&self, request: DocumentSymbolRequest) -> ExtractResult<DocumentSymbolExtraction> {
        let request = validate_document_symbol_request(request)?;
        let mut batch = self
            .run_batch(DocumentSymbolBatchRequest {
                workspace_root: request.workspace_root,
                package_path: request.package_path,
                file_paths: vec![request.file_path],
            })
            .await?;

        batch.extractions.pop().ok_or_else(|| {
            ExtractError::response_shape(
                self.provider_id().as_str(),
                "textDocument/documentSymbol",
                "single-file document symbol extraction returned no files",
            )
        })
    }

    async fn run_batch(
        &self,
        request: DocumentSymbolBatchRequest,
    ) -> ExtractResult<DocumentSymbolBatchExtraction> {
        let request = validate_document_symbol_batch_request(request)?;
        let provider_version = rust_analyzer_lib::provider_version();
        let document_symbols = rust_analyzer_lib::document_symbols_for_files(
            &request.workspace_root,
            &request.file_paths,
        )
        .map_err(|source| facade_error("rust-analyzer-lib document_symbols_for_files", source))?;
        let mut extractions = Vec::with_capacity(document_symbols.len());

        for (file_path, symbols) in document_symbols {
            let response = DocumentSymbolResponse::Nested(symbols.clone());
            let extraction = RustDocumentSymbolMapper::map_response(
                DocumentSymbolRequest {
                    workspace_root: request.workspace_root.clone(),
                    package_path: request.package_path.clone(),
                    file_path: file_path.clone(),
                },
                response,
                provider_version.clone(),
                json!({
                    "facade": "rust-analyzer-lib",
                    "lsp_method": "textDocument/documentSymbol",
                    "document_symbol": symbols
                }),
            )?;

            extractions.push(extraction);
        }

        Ok(DocumentSymbolBatchExtraction {
            provider: self.provider_id(),
            provider_version,
            extractions,
            raw_metadata: json!({
                "facade": "rust-analyzer-lib",
                "file_count": request.file_paths.len(),
            }),
        })
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
    ) -> ExtractResult<DocumentSymbolExtraction> {
        self.run(request).await
    }
}

fn facade_error(
    context: &'static str,
    source: rust_analyzer_lib::RustAnalyzerLibError,
) -> ExtractError {
    ExtractError::rust_analyzer_lib(context, source)
}
