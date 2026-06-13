use std::collections::HashMap;

use semantic_graph_store::{
    EdgeEvidenceInput, EdgeInput, FileInput, GraphStore, NodeInput, OccurrenceInput,
};
use serde_json::{Value, json};

use crate::document_symbols::paths::basename_from_relative_path;
use crate::error::{ExtractError, Result};
use crate::model::DocumentSymbolExtraction;
use crate::persist::PersistenceSummary;

pub struct ExtractionPersister;

impl ExtractionPersister {
    pub async fn persist_document_symbols(
        &self,
        store: &GraphStore,
        workspace_root_uri: &str,
        extraction: &DocumentSymbolExtraction,
    ) -> Result<PersistenceSummary> {
        let workspace_id = store
            .create_workspace(
                workspace_root_uri,
                extraction.source_file.language.workspace_kind(),
            )
            .await?;
        let run_id = store
            .start_run(
                workspace_id,
                extraction.provider.as_str(),
                extraction.provider_version.as_deref(),
                None,
            )
            .await?;

        let result = self
            .persist_after_run_started(store, workspace_id, run_id, extraction)
            .await;

        match result {
            Ok(summary) => {
                store.finish_run(run_id, "complete").await?;
                Ok(summary)
            }
            Err(error) => {
                let finish_result = store.finish_run(run_id, "failed").await;
                if let Err(finish_error) = finish_result {
                    return Err(ExtractError::Storage(finish_error));
                }
                Err(error)
            }
        }
    }

    async fn persist_after_run_started(
        &self,
        store: &GraphStore,
        workspace_id: i64,
        run_id: i64,
        extraction: &DocumentSymbolExtraction,
    ) -> Result<PersistenceSummary> {
        let file_id = store
            .upsert_file(FileInput {
                workspace_id,
                uri: &extraction.source_file.uri,
                path: &extraction.source_file.relative_path,
                language: extraction.source_file.language.as_store_str(),
                content_hash: extraction.source_file.content_hash.as_deref(),
                last_seen_run_id: Some(run_id),
                properties_json: json!({
                    "provider": extraction.provider.as_str(),
                    "language": extraction.source_file.language.as_store_str(),
                    "raw_metadata": extraction.raw_metadata,
                }),
            })
            .await?;

        let mut node_ids = HashMap::new();
        let file_node_id = self
            .upsert_file_node(store, workspace_id, run_id, file_id, extraction)
            .await?;
        node_ids.insert(extraction.source_file.file_symbol_key.clone(), file_node_id);

        for symbol in &extraction.symbols {
            let container_node_id = if let Some(parent_symbol_key) = &symbol.parent_symbol_key {
                Some(
                    node_ids
                        .get(parent_symbol_key)
                        .ok_or_else(|| {
                            ExtractError::response_shape(
                                extraction.provider.as_str(),
                                "textDocument/documentSymbol",
                                format!(
                                    "symbol {} references missing parent {}",
                                    symbol.symbol_key, parent_symbol_key
                                ),
                            )
                        })?
                        .clone(),
                )
            } else {
                None
            };

            let node_id = store
                .upsert_node(NodeInput {
                    workspace_id,
                    language: symbol.language.as_store_str(),
                    kind: &symbol.kind,
                    name: &symbol.name,
                    qualified_name: symbol.qualified_name.as_deref(),
                    display_name: Some(&symbol.name),
                    symbol_key: &symbol.symbol_key,
                    file_id: Some(file_id),
                    range: Some(symbol.range),
                    selection_range: Some(symbol.selection_range),
                    container_node_id: container_node_id.as_deref(),
                    properties_json: symbol_properties_json(symbol),
                    run_id: Some(run_id),
                })
                .await?;

            store
                .insert_occurrence(OccurrenceInput {
                    node_id: &node_id,
                    run_id,
                    file_id,
                    role: "definition",
                    range: symbol.selection_range,
                    enclosing_node_id: container_node_id.as_deref(),
                    raw_json: Some(symbol.raw_json.clone()),
                })
                .await?;

            node_ids.insert(symbol.symbol_key.clone(), node_id);
        }

        for relation in &extraction.relations {
            let src_node_id = node_ids
                .get(&relation.source_symbol_key)
                .ok_or_else(|| {
                    ExtractError::response_shape(
                        extraction.provider.as_str(),
                        "textDocument/documentSymbol",
                        format!(
                            "relation source {} was not persisted",
                            relation.source_symbol_key
                        ),
                    )
                })?
                .clone();
            let dst_node_id = node_ids
                .get(&relation.target_symbol_key)
                .ok_or_else(|| {
                    ExtractError::response_shape(
                        extraction.provider.as_str(),
                        "textDocument/documentSymbol",
                        format!(
                            "relation target {} was not persisted",
                            relation.target_symbol_key
                        ),
                    )
                })?
                .clone();

            let edge_id = store
                .upsert_edge(EdgeInput {
                    workspace_id,
                    src_node_id: &src_node_id,
                    dst_node_id: &dst_node_id,
                    relation: &relation.relation,
                    context: None,
                    confidence: &relation.confidence,
                    confidence_score: relation.confidence_score,
                    weight: 1.0,
                    properties_json: json!({
                        "provider": relation.provider.as_str(),
                        "source": "textDocument/documentSymbol",
                    }),
                    run_id: Some(run_id),
                })
                .await?;

            store
                .insert_edge_evidence(EdgeEvidenceInput {
                    edge_id: &edge_id,
                    run_id,
                    provider: relation.provider.as_str(),
                    lsp_method: Some("textDocument/documentSymbol"),
                    file_id: Some(file_id),
                    range: relation.range,
                    raw_json: Some(relation.raw_json.clone()),
                })
                .await?;
        }

        Ok(PersistenceSummary {
            workspace_id,
            run_id,
            files: 1,
            nodes: node_ids.len(),
            edges: extraction.relations.len(),
            occurrences: extraction.symbols.len(),
            evidence: extraction.relations.len(),
        })
    }

    async fn upsert_file_node(
        &self,
        store: &GraphStore,
        workspace_id: i64,
        run_id: i64,
        file_id: i64,
        extraction: &DocumentSymbolExtraction,
    ) -> Result<String> {
        let file_name = basename_from_relative_path(&extraction.source_file.relative_path);

        store
            .upsert_node(NodeInput {
                workspace_id,
                language: extraction.source_file.language.as_store_str(),
                kind: "file",
                name: &file_name,
                qualified_name: Some(&extraction.source_file.relative_path),
                display_name: Some(&file_name),
                symbol_key: &extraction.source_file.file_symbol_key,
                file_id: Some(file_id),
                range: None,
                selection_range: None,
                container_node_id: None,
                properties_json: json!({
                    "provider": extraction.provider.as_str(),
                    "language": extraction.source_file.language.as_store_str(),
                    "uri": extraction.source_file.uri,
                }),
                run_id: Some(run_id),
            })
            .await
            .map_err(ExtractError::Storage)
    }
}

fn symbol_properties_json(symbol: &crate::model::ExtractedSymbol) -> Value {
    json!({
        "provider": symbol.provider.as_str(),
        "language": symbol.language.as_store_str(),
        "lsp_kind": symbol.raw_json.get("lsp_kind").cloned(),
        "detail": symbol.detail.as_deref(),
        "raw": symbol.raw_json,
    })
}
