use crate::{
    CSharpLsLibError, CSharpLsLibResult,
    lsp::{CSharpLspClient, LaunchConfig, file_uri, path_from_file_uri},
    model::{
        FileSemanticResult, FileSemanticWork, ResolvedCallTarget, ResolvedIncomingCall,
        ResolvedIncomingCallSet, ResolvedReferenceLocation, ResolvedReferenceSet,
        ResolvedReferenceTarget,
    },
    semantic::ProgressCallback,
};

use lsp_types::{CallHierarchyIncomingCall, CallHierarchyItem, DocumentSymbolResponse, Location};
use serde_json::json;
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};
use tokio::time::{Duration, sleep};

pub struct CSharpLsWorker {
    client: CSharpLspClient,
    opened_documents: HashSet<PathBuf>,
}

impl CSharpLsWorker {
    pub async fn start(
        binary: PathBuf,
        solution: PathBuf,
        log_level: String,
        features: Vec<String>,
        startup_timeout_ms: u64,
        request_timeout_ms: u64,
    ) -> CSharpLsLibResult<Self> {
        let config = LaunchConfig {
            binary,
            solution,
            log_level,
            features,
            startup_timeout_ms,
            request_timeout_ms,
        };
        let mut client = CSharpLspClient::spawn(config)?;
        client.initialize().await?;
        Ok(Self {
            client,
            opened_documents: HashSet::new(),
        })
    }

    pub async fn document_symbols_for_files(
        &mut self,
        file_paths: Vec<PathBuf>,
    ) -> CSharpLsLibResult<Vec<(PathBuf, Vec<lsp_types::DocumentSymbol>)>> {
        self.document_symbols_for_files_internal(file_paths, None)
            .await
    }

    pub async fn document_symbols_for_files_with_progress(
        &mut self,
        file_paths: Vec<PathBuf>,
        progress: ProgressCallback,
    ) -> CSharpLsLibResult<Vec<(PathBuf, Vec<lsp_types::DocumentSymbol>)>> {
        self.document_symbols_for_files_internal(file_paths, Some(progress))
            .await
    }

    async fn document_symbols_for_files_internal(
        &mut self,
        file_paths: Vec<PathBuf>,
        progress: Option<ProgressCallback>,
    ) -> CSharpLsLibResult<Vec<(PathBuf, Vec<lsp_types::DocumentSymbol>)>> {
        let mut results = Vec::with_capacity(file_paths.len());
        for file_path in file_paths {
            self.open_document(&file_path).await?;
            let symbols = self.document_symbols_for_open_file(&file_path).await?;
            if let Some(progress) = &progress {
                progress();
            }
            results.push((file_path, symbols));
        }

        Ok(results)
    }

    async fn document_symbols_for_open_file(
        &mut self,
        file_path: &Path,
    ) -> CSharpLsLibResult<Vec<lsp_types::DocumentSymbol>> {
        let uri = file_uri(file_path)?;
        let response_value: serde_json::Value = self
            .client
            .request(
                "textDocument/documentSymbol",
                json!({
                    "textDocument": {
                        "uri": uri,
                    },
                }),
            )
            .await?;
        let response_value = normalize_document_symbol_response(response_value);
        let response: Option<DocumentSymbolResponse> = serde_json::from_value(response_value)
            .map_err(|source| {
                CSharpLsLibError::json("deserialize documentSymbol response", source)
            })?;
        match response {
            Some(DocumentSymbolResponse::Nested(symbols)) => Ok(symbols),
            Some(DocumentSymbolResponse::Flat(_)) => Err(CSharpLsLibError::response_shape(
                "textDocument/documentSymbol",
                "expected hierarchical DocumentSymbol[] but received SymbolInformation[]",
            )),
            None => Ok(Vec::new()),
        }
    }

    pub async fn references_for_symbol(
        &mut self,
        target: &ResolvedReferenceTarget,
    ) -> CSharpLsLibResult<ResolvedReferenceSet> {
        self.open_document(&target.file_path).await?;
        let uri = file_uri(&target.file_path)?;
        let locations: Option<Vec<Location>> = self
            .client
            .request(
                "textDocument/references",
                json!({
                    "textDocument": {
                        "uri": uri,
                    },
                    "position": target.selection_range.start,
                    "context": {
                        "includeDeclaration": false,
                    },
                }),
            )
            .await?;
        let references = locations
            .unwrap_or_default()
            .into_iter()
            .map(reference_location)
            .collect::<CSharpLsLibResult<Vec<_>>>()?;

        Ok(ResolvedReferenceSet {
            target_file_path: target.file_path.clone(),
            target_selection_range: target.selection_range,
            references,
        })
    }

    pub async fn incoming_calls_for_symbol(
        &mut self,
        target: &ResolvedCallTarget,
    ) -> CSharpLsLibResult<ResolvedIncomingCallSet> {
        self.open_document(&target.file_path).await?;
        let uri = file_uri(&target.file_path)?;
        let items: Option<Vec<CallHierarchyItem>> = self
            .client
            .request(
                "textDocument/prepareCallHierarchy",
                json!({
                    "textDocument": {
                        "uri": uri,
                    },
                    "position": target.selection_range.start,
                }),
            )
            .await?;
        let Some(items) = items else {
            return Ok(ResolvedIncomingCallSet {
                target_file_path: target.file_path.clone(),
                target_selection_range: target.selection_range,
                incoming_calls: Vec::new(),
                skipped_non_callable_prepare_items: 1,
            });
        };
        if items.is_empty() {
            return Ok(ResolvedIncomingCallSet {
                target_file_path: target.file_path.clone(),
                target_selection_range: target.selection_range,
                incoming_calls: Vec::new(),
                skipped_non_callable_prepare_items: 1,
            });
        }

        let mut incoming_calls = Vec::new();
        for item in items {
            let calls: Option<Vec<CallHierarchyIncomingCall>> = self
                .client
                .request(
                    "callHierarchy/incomingCalls",
                    json!({
                        "item": item,
                    }),
                )
                .await?;
            for incoming_call in calls.unwrap_or_default() {
                incoming_calls.push(map_incoming_call(incoming_call)?);
            }
        }

        Ok(ResolvedIncomingCallSet {
            target_file_path: target.file_path.clone(),
            target_selection_range: target.selection_range,
            incoming_calls,
            skipped_non_callable_prepare_items: 0,
        })
    }

    pub async fn file_semantic_work(
        &mut self,
        work: FileSemanticWork,
    ) -> CSharpLsLibResult<FileSemanticResult> {
        let mut reference_sets = Vec::with_capacity(work.reference_targets.len());
        for target in &work.reference_targets {
            reference_sets.push(self.references_for_symbol(target).await?);
        }

        let mut incoming_call_sets = Vec::with_capacity(work.call_targets.len());
        for target in &work.call_targets {
            incoming_call_sets.push(self.incoming_calls_for_symbol(target).await?);
        }

        Ok(FileSemanticResult {
            reference_sets,
            incoming_call_sets,
        })
    }

    pub async fn shutdown(self) -> CSharpLsLibResult<()> {
        self.client.shutdown().await
    }

    async fn open_document(&mut self, file_path: &PathBuf) -> CSharpLsLibResult<()> {
        if self.opened_documents.contains(file_path) {
            return Ok(());
        }
        let text = fs::read_to_string(file_path).map_err(|source| {
            CSharpLsLibError::io(
                "read C# source file for didOpen",
                Some(file_path.clone()),
                source,
            )
        })?;
        let uri = file_uri(file_path)?;
        self.client
            .notify(
                "textDocument/didOpen",
                Some(json!({
                    "textDocument": {
                        "uri": uri,
                        "languageId": "csharp",
                        "version": 1,
                        "text": text,
                    },
                })),
            )
            .await
            .map(|()| {
                self.opened_documents.insert(file_path.clone());
            })?;
        sleep(Duration::from_millis(250)).await;
        Ok(())
    }
}

fn reference_location(location: Location) -> CSharpLsLibResult<ResolvedReferenceLocation> {
    Ok(ResolvedReferenceLocation {
        file_path: path_from_file_uri(&location.uri)?,
        range: location.range,
    })
}

fn map_incoming_call(
    incoming_call: CallHierarchyIncomingCall,
) -> CSharpLsLibResult<ResolvedIncomingCall> {
    Ok(ResolvedIncomingCall {
        caller_file_path: path_from_file_uri(&incoming_call.from.uri)?,
        caller_name: incoming_call.from.name,
        caller_kind: format!("{:?}", incoming_call.from.kind),
        caller_range: incoming_call.from.range,
        caller_selection_range: incoming_call.from.selection_range,
        from_ranges: incoming_call.from_ranges,
    })
}

fn normalize_document_symbol_response(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Array(items) => serde_json::Value::Array(
            items
                .into_iter()
                .map(normalize_document_symbol_value)
                .collect(),
        ),
        other => other,
    }
}

fn normalize_document_symbol_value(value: serde_json::Value) -> serde_json::Value {
    let serde_json::Value::Object(mut object) = value else {
        return value;
    };

    rename_key(&mut object, "Name", "name");
    rename_key(&mut object, "Detail", "detail");
    rename_key(&mut object, "Kind", "kind");
    rename_key(&mut object, "Range", "range");
    rename_key(&mut object, "SelectionRange", "selectionRange");
    rename_key(&mut object, "Tags", "tags");
    rename_key(&mut object, "Deprecated", "deprecated");
    if let Some(children) = object
        .remove("Children")
        .or_else(|| object.remove("children"))
    {
        object.insert(
            "children".to_string(),
            match children {
                serde_json::Value::Array(items) => serde_json::Value::Array(
                    items
                        .into_iter()
                        .map(normalize_document_symbol_value)
                        .collect(),
                ),
                other => other,
            },
        );
    }
    if let Some(range) = object.remove("range") {
        object.insert("range".to_string(), normalize_range_value(range));
    }
    if let Some(selection_range) = object.remove("selectionRange") {
        object.insert(
            "selectionRange".to_string(),
            normalize_range_value(selection_range),
        );
    }

    serde_json::Value::Object(object)
}

fn normalize_range_value(value: serde_json::Value) -> serde_json::Value {
    let serde_json::Value::Object(mut object) = value else {
        return value;
    };

    rename_key(&mut object, "Start", "start");
    rename_key(&mut object, "End", "end");
    if let Some(start) = object.remove("start") {
        object.insert("start".to_string(), normalize_position_value(start));
    }
    if let Some(end) = object.remove("end") {
        object.insert("end".to_string(), normalize_position_value(end));
    }

    serde_json::Value::Object(object)
}

fn normalize_position_value(value: serde_json::Value) -> serde_json::Value {
    let serde_json::Value::Object(mut object) = value else {
        return value;
    };

    rename_key(&mut object, "Line", "line");
    rename_key(&mut object, "Character", "character");

    serde_json::Value::Object(object)
}

fn rename_key(object: &mut serde_json::Map<String, serde_json::Value>, from: &str, to: &str) {
    if object.contains_key(to) {
        return;
    }
    if let Some(value) = object.remove(from) {
        object.insert(to.to_string(), value);
    }
}
