use crate::{
    dto::{
        GraphEdgeDetailsDto, GraphNodeDetailsDto, GraphNodeSearchResultsDto, JsonRpcResponseDto,
    },
    query::GraphQueryService,
    rpc::rpc_handler,
    server::AppState,
};

use axum::{Json, body::Bytes, extract::State};
use semantic_graph_db_manager::{
    EdgeEvidenceInput, EdgeInput, FileInput, NodeInput, OccurrenceInput, TextRange, WriteManager,
    edge_id, node_id,
};
use serde_json::{Value, json};
use std::{
    error::Error,
    io,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static TEMP_DATABASE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[tokio::test]
async fn projection_includes_selected_symbols_files_and_edges() -> Result<(), Box<dyn Error>> {
    let database_path = seeded_database_path().await?;

    let service = GraphQueryService::new(database_path.clone());
    let projection = service.projection(1).await?;

    assert_eq!(2, projection.nodes.len());
    assert_eq!(1, projection.edges.len());
    assert_eq!(1, projection.metadata.edge_count);
    assert_eq!(2, projection.metadata.node_count);
    assert!(projection.nodes.iter().any(|node| node.kind == "file"));
    assert!(
        projection
            .nodes
            .iter()
            .any(|node| node.display_label == "run")
    );
    assert_eq!("contains", projection.edges[0].relation);

    std::fs::remove_file(database_path)?;
    Ok(())
}

#[tokio::test]
async fn projection_includes_reference_edges_when_endpoints_are_selected()
-> Result<(), Box<dyn Error>> {
    let database_path = seeded_database_path().await?;

    let service = GraphQueryService::new(database_path.clone());
    let projection = service.projection(3).await?;

    assert!(
        projection
            .edges
            .iter()
            .any(|edge| edge.relation == "references")
    );

    std::fs::remove_file(database_path)?;
    Ok(())
}

#[tokio::test]
async fn projection_includes_call_edges_when_endpoints_are_selected() -> Result<(), Box<dyn Error>>
{
    let database_path = seeded_database_path().await?;

    let service = GraphQueryService::new(database_path.clone());
    let projection = service.projection(3).await?;

    assert!(projection.edges.iter().any(|edge| edge.relation == "calls"));

    std::fs::remove_file(database_path)?;
    Ok(())
}

#[tokio::test]
async fn node_details_returns_metadata_occurrences_and_counts() -> Result<(), Box<dyn Error>> {
    let database_path = seeded_database_path().await?;
    let ids = FixtureIds::new();

    let service = GraphQueryService::new(database_path.clone());
    let details = service.node_details(&ids.symbol_run).await?;

    assert_eq!(ids.symbol_run, details.node_id);
    assert_eq!("function", details.kind);
    assert_eq!("run", details.display_label);
    assert_eq!(Some("crate::run"), details.qualified_name.as_deref());
    assert_eq!(Some("src/lib.rs"), details.source_file_path.as_deref());
    assert_eq!(
        Some(ids.module_root.as_str()),
        details.container_node_id.as_deref()
    );
    assert_eq!(Some("crate"), details.container_display_label.as_deref());
    assert_eq!(2, details.incoming_edge_count);
    assert_eq!(3, details.outgoing_edge_count);
    assert_eq!(4, details.relations.len());
    assert!(
        details
            .relations
            .iter()
            .any(|relation| relation.direction == "incoming"
                && relation.relation == "contains"
                && relation.edge_count == 2)
    );
    assert!(
        details
            .relations
            .iter()
            .any(|relation| relation.direction == "outgoing"
                && relation.relation == "contains"
                && relation.edge_count == 1)
    );
    assert!(
        details
            .relations
            .iter()
            .any(|relation| relation.direction == "outgoing"
                && relation.relation == "references"
                && relation.edge_count == 1)
    );
    assert!(
        details
            .relations
            .iter()
            .any(|relation| relation.direction == "outgoing"
                && relation.relation == "calls"
                && relation.edge_count == 1)
    );
    assert_eq!(1, details.occurrences.len());
    assert_eq!("definition", details.occurrences[0].role);
    assert_eq!("src/lib.rs", details.occurrences[0].source_file_path);
    assert_eq!(json!({ "visibility": "public" }), details.properties_json);

    std::fs::remove_file(database_path)?;
    Ok(())
}

#[tokio::test]
async fn edge_details_returns_reference_context_weight_and_evidence() -> Result<(), Box<dyn Error>>
{
    let database_path = seeded_database_path().await?;
    let ids = FixtureIds::new();

    let service = GraphQueryService::new(database_path.clone());
    let details = service.edge_details(&ids.edge_run_helper_reference).await?;

    assert_eq!(ids.edge_run_helper_reference, details.edge_id);
    assert_eq!("references", details.relation);
    assert_eq!(Some("symbol"), details.context.as_deref());
    assert_eq!("EXTRACTED", details.confidence);
    assert_eq!(2.0, details.weight);
    assert_eq!(ids.symbol_run, details.source.node_id);
    assert_eq!(ids.symbol_helper, details.target.node_id);
    assert_eq!(1, details.evidence.len());
    assert_eq!("rust-analyzer", details.evidence[0].provider);
    assert_eq!(
        Some("textDocument/references"),
        details.evidence[0].lsp_method.as_deref()
    );
    assert_eq!(
        Some("src/lib.rs"),
        details.evidence[0].source_file_path.as_deref()
    );

    std::fs::remove_file(database_path)?;
    Ok(())
}

#[tokio::test]
async fn edge_details_returns_call_context_weight_and_evidence() -> Result<(), Box<dyn Error>> {
    let database_path = seeded_database_path().await?;
    let ids = FixtureIds::new();

    let service = GraphQueryService::new(database_path.clone());
    let details = service.edge_details(&ids.edge_run_helper_call).await?;

    assert_eq!(ids.edge_run_helper_call, details.edge_id);
    assert_eq!("calls", details.relation);
    assert_eq!(Some("direct"), details.context.as_deref());
    assert_eq!("EXTRACTED", details.confidence);
    assert_eq!(2.0, details.weight);
    assert_eq!(ids.symbol_run, details.source.node_id);
    assert_eq!(ids.symbol_helper, details.target.node_id);
    assert_eq!(1, details.evidence.len());
    assert_eq!("rust-analyzer", details.evidence[0].provider);
    assert_eq!(
        Some("callHierarchy/outgoingCalls"),
        details.evidence[0].lsp_method.as_deref()
    );
    assert_eq!(
        Some("src/lib.rs"),
        details.evidence[0].source_file_path.as_deref()
    );

    std::fs::remove_file(database_path)?;
    Ok(())
}

#[tokio::test]
async fn edge_details_returns_endpoints_and_evidence() -> Result<(), Box<dyn Error>> {
    let database_path = seeded_database_path().await?;
    let ids = FixtureIds::new();

    let service = GraphQueryService::new(database_path.clone());
    let details = service.edge_details(&ids.edge_file_run).await?;

    assert_eq!(ids.edge_file_run, details.edge_id);
    assert_eq!("contains", details.relation);
    assert_eq!("EXTRACTED", details.confidence);
    assert_eq!(ids.file_src_lib, details.source.node_id);
    assert_eq!("lib.rs", details.source.display_label);
    assert_eq!(ids.symbol_run, details.target.node_id);
    assert_eq!("run", details.target.display_label);
    assert_eq!(1, details.evidence.len());
    assert_eq!("rust-analyzer", details.evidence[0].provider);
    assert_eq!(
        Some("textDocument/documentSymbol"),
        details.evidence[0].lsp_method.as_deref()
    );
    assert_eq!(
        Some("src/lib.rs"),
        details.evidence[0].source_file_path.as_deref()
    );
    assert_eq!(
        json!({ "source": "edge-evidence" }),
        details.evidence[0].raw_json.clone().unwrap_or(Value::Null)
    );

    std::fs::remove_file(database_path)?;
    Ok(())
}

#[tokio::test]
async fn search_nodes_finds_name_qualified_name_and_file_path() -> Result<(), Box<dyn Error>> {
    let database_path = seeded_database_path().await?;
    let ids = FixtureIds::new();

    let service = GraphQueryService::new(database_path.clone());

    let by_name = service.search_nodes("run", 25).await?;
    assert!(
        by_name
            .results
            .iter()
            .any(|result| result.node_id == ids.symbol_run)
    );

    let by_qualified_name = service.search_nodes("crate::run", 25).await?;
    assert!(
        by_qualified_name
            .results
            .iter()
            .any(|result| result.node_id == ids.symbol_run)
    );

    let by_file_path = service.search_nodes("z.rs", 25).await?;
    assert!(
        by_file_path
            .results
            .iter()
            .any(|result| result.node_id == ids.symbol_helper)
    );

    let limited = service.search_nodes("r", 1).await?;
    assert_eq!(1, limited.results.len());

    let missing = service.search_nodes("does-not-exist", 25).await?;
    assert!(missing.results.is_empty());

    std::fs::remove_file(database_path)?;
    Ok(())
}

#[tokio::test]
async fn rpc_invalid_params_and_missing_ids_return_json_rpc_errors() -> Result<(), Box<dyn Error>> {
    let database_path = seeded_database_path().await?;
    let ids = FixtureIds::new();
    let state = AppState::new(database_path.clone());

    let blank_node_id = rpc_request(
        state.clone(),
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "graph.node_details",
            "params": { "nodeId": " " }
        }),
    )
    .await?;
    assert_rpc_error(&blank_node_id, -32602, "nodeId must not be blank")?;

    let blank_edge_id = rpc_request(
        state.clone(),
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "graph.edge_details",
            "params": { "edgeId": " " }
        }),
    )
    .await?;
    assert_rpc_error(&blank_edge_id, -32602, "edgeId must not be blank")?;

    let blank_search = rpc_request(
        state.clone(),
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "graph.search_nodes",
            "params": { "query": " " }
        }),
    )
    .await?;
    assert_rpc_error(&blank_search, -32602, "query must not be blank")?;

    let invalid_search_limit = rpc_request(
        state.clone(),
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "graph.search_nodes",
            "params": { "query": "run", "limit": 51 }
        }),
    )
    .await?;
    assert_rpc_error(
        &invalid_search_limit,
        -32602,
        "limit must be between 1 and 50",
    )?;

    let missing_node = rpc_request(
        state.clone(),
        json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "graph.node_details",
            "params": { "nodeId": "missing-node" }
        }),
    )
    .await?;
    assert_rpc_error(&missing_node, -32004, "node 'missing-node' not found")?;

    let missing_edge = rpc_request(
        state.clone(),
        json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "graph.edge_details",
            "params": { "edgeId": "missing-edge" }
        }),
    )
    .await?;
    assert_rpc_error(&missing_edge, -32004, "edge 'missing-edge' not found")?;

    let node_details = rpc_request(
        state.clone(),
        json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "graph.node_details",
            "params": { "nodeId": ids.symbol_run.as_str() }
        }),
    )
    .await?;
    let node_details: GraphNodeDetailsDto = result_value(node_details)?;
    assert_eq!(ids.symbol_run, node_details.node_id);

    let edge_details = rpc_request(
        state.clone(),
        json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "graph.edge_details",
            "params": { "edgeId": ids.edge_file_run.as_str() }
        }),
    )
    .await?;
    let edge_details: GraphEdgeDetailsDto = result_value(edge_details)?;
    assert_eq!(ids.edge_file_run, edge_details.edge_id);

    let search_results = rpc_request(
        state,
        json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "graph.search_nodes",
            "params": { "query": "run", "limit": 25 }
        }),
    )
    .await?;
    let search_results: GraphNodeSearchResultsDto = result_value(search_results)?;
    assert!(
        search_results
            .results
            .iter()
            .any(|result| result.node_id == ids.symbol_run)
    );

    std::fs::remove_file(database_path)?;
    Ok(())
}

async fn rpc_request(
    state: AppState,
    request: Value,
) -> Result<JsonRpcResponseDto, Box<dyn Error>> {
    let Json(response) = rpc_handler(State(state), Bytes::from(request.to_string())).await;
    Ok(response)
}

fn assert_rpc_error(
    response: &JsonRpcResponseDto,
    code: i64,
    message: &str,
) -> Result<(), Box<dyn Error>> {
    let error = response
        .error
        .as_ref()
        .ok_or_else(|| io::Error::other("expected JSON-RPC error"))?;

    assert_eq!(code, error.code);
    assert_eq!(message, error.message);
    Ok(())
}

fn result_value<T>(response: JsonRpcResponseDto) -> Result<T, Box<dyn Error>>
where
    T: serde::de::DeserializeOwned,
{
    let value = response
        .result
        .ok_or_else(|| io::Error::other("expected JSON-RPC result"))?;
    Ok(serde_json::from_value(value)?)
}

async fn seeded_database_path() -> Result<PathBuf, Box<dyn Error>> {
    let database_path = temp_database_path()?;
    let writer = WriteManager::start(&database_path).await?;
    writer.migrate().await?;
    seed_fixture_database(&writer).await?;
    writer.shutdown().await?;
    Ok(database_path)
}

async fn seed_fixture_database(
    writer: &semantic_graph_db_manager::WriteHandle,
) -> Result<(), Box<dyn Error>> {
    let workspace_id = writer.create_workspace("file:///fixture", "rust").await?;
    let run_id = writer
        .start_run(workspace_id, "rust-analyzer", Some("fixture"), None)
        .await?;
    let lib_file_id = writer
        .upsert_file(FileInput {
            workspace_id,
            uri: "file:///fixture/src/lib.rs",
            path: "src/lib.rs",
            language: "rust",
            content_hash: None,
            last_seen_run_id: Some(run_id),
            properties_json: json!({}),
        })
        .await?;
    let helper_file_id = writer
        .upsert_file(FileInput {
            workspace_id,
            uri: "file:///fixture/src/z.rs",
            path: "src/z.rs",
            language: "rust",
            content_hash: None,
            last_seen_run_id: Some(run_id),
            properties_json: json!({}),
        })
        .await?;
    let ids = FixtureIds::new();

    writer
        .upsert_node(NodeInput {
            workspace_id,
            language: "rust",
            kind: "file",
            name: "lib.rs",
            qualified_name: Some("src/lib.rs"),
            display_name: Some("lib.rs"),
            symbol_key: "file:///fixture/src/lib.rs",
            file_id: Some(lib_file_id),
            range: None,
            selection_range: None,
            container_node_id: None,
            properties_json: json!({}),
            run_id: Some(run_id),
        })
        .await?;
    writer
        .upsert_node(NodeInput {
            workspace_id,
            language: "rust",
            kind: "module",
            name: "crate",
            qualified_name: Some("crate"),
            display_name: Some("crate"),
            symbol_key: "module:crate",
            file_id: Some(lib_file_id),
            range: Some(range(1, 0, 30, 0)),
            selection_range: Some(range(1, 0, 1, 0)),
            container_node_id: None,
            properties_json: json!({}),
            run_id: Some(run_id),
        })
        .await?;
    writer
        .upsert_node(NodeInput {
            workspace_id,
            language: "rust",
            kind: "function",
            name: "run",
            qualified_name: Some("crate::run"),
            display_name: Some("run"),
            symbol_key: "function:crate::run",
            file_id: Some(lib_file_id),
            range: Some(range(10, 4, 12, 5)),
            selection_range: Some(range(10, 7, 10, 10)),
            container_node_id: Some(&ids.module_root),
            properties_json: json!({ "visibility": "public" }),
            run_id: Some(run_id),
        })
        .await?;
    writer
        .upsert_node(NodeInput {
            workspace_id,
            language: "rust",
            kind: "function",
            name: "helper",
            qualified_name: Some("crate::z_helper"),
            display_name: Some("helper"),
            symbol_key: "function:crate::z_helper",
            file_id: Some(helper_file_id),
            range: Some(range(2, 0, 4, 1)),
            selection_range: Some(range(2, 3, 2, 9)),
            container_node_id: None,
            properties_json: json!({}),
            run_id: Some(run_id),
        })
        .await?;

    writer
        .upsert_edge(EdgeInput {
            workspace_id,
            src_node_id: &ids.file_src_lib,
            dst_node_id: &ids.symbol_run,
            relation: "contains",
            context: Some("document-symbol"),
            confidence: "EXTRACTED",
            confidence_score: 1.0,
            weight: 1.0,
            properties_json: json!({ "source": "documentSymbol" }),
            run_id: Some(run_id),
        })
        .await?;
    writer
        .upsert_edge(EdgeInput {
            workspace_id,
            src_node_id: &ids.module_root,
            dst_node_id: &ids.symbol_run,
            relation: "contains",
            context: Some("document-symbol"),
            confidence: "EXTRACTED",
            confidence_score: 1.0,
            weight: 1.0,
            properties_json: json!({}),
            run_id: Some(run_id),
        })
        .await?;
    writer
        .upsert_edge(EdgeInput {
            workspace_id,
            src_node_id: &ids.symbol_run,
            dst_node_id: &ids.symbol_helper,
            relation: "contains",
            context: Some("document-symbol"),
            confidence: "EXTRACTED",
            confidence_score: 1.0,
            weight: 1.0,
            properties_json: json!({}),
            run_id: Some(run_id),
        })
        .await?;
    writer
        .upsert_edge(EdgeInput {
            workspace_id,
            src_node_id: &ids.symbol_run,
            dst_node_id: &ids.symbol_helper,
            relation: "references",
            context: Some("symbol"),
            confidence: "EXTRACTED",
            confidence_score: 1.0,
            weight: 2.0,
            properties_json: json!({ "source_resolution": "symbol" }),
            run_id: Some(run_id),
        })
        .await?;
    writer
        .upsert_edge(EdgeInput {
            workspace_id,
            src_node_id: &ids.symbol_run,
            dst_node_id: &ids.symbol_helper,
            relation: "calls",
            context: Some("direct"),
            confidence: "EXTRACTED",
            confidence_score: 1.0,
            weight: 2.0,
            properties_json: json!({ "source_resolution": "symbol" }),
            run_id: Some(run_id),
        })
        .await?;

    writer
        .insert_occurrence(OccurrenceInput {
            node_id: &ids.symbol_run,
            run_id,
            file_id: lib_file_id,
            role: "definition",
            range: range(10, 4, 12, 5),
            enclosing_node_id: Some(&ids.module_root),
            raw_json: Some(json!({ "source": "occurrence" })),
        })
        .await?;
    writer
        .insert_occurrence(OccurrenceInput {
            node_id: &ids.symbol_helper,
            run_id,
            file_id: lib_file_id,
            role: "reference",
            range: range(11, 8, 11, 14),
            enclosing_node_id: Some(&ids.symbol_run),
            raw_json: Some(json!({ "source": "reference-occurrence" })),
        })
        .await?;
    writer
        .insert_occurrence(OccurrenceInput {
            node_id: &ids.symbol_helper,
            run_id,
            file_id: lib_file_id,
            role: "call",
            range: range(11, 8, 11, 14),
            enclosing_node_id: Some(&ids.symbol_run),
            raw_json: Some(json!({ "source": "call-occurrence" })),
        })
        .await?;

    writer
        .insert_edge_evidence(EdgeEvidenceInput {
            edge_id: &ids.edge_file_run,
            run_id,
            provider: "rust-analyzer",
            lsp_method: Some("textDocument/documentSymbol"),
            file_id: Some(lib_file_id),
            range: Some(range(10, 4, 12, 5)),
            raw_json: Some(json!({ "source": "edge-evidence" })),
        })
        .await?;
    writer
        .insert_edge_evidence(EdgeEvidenceInput {
            edge_id: &ids.edge_run_helper_reference,
            run_id,
            provider: "rust-analyzer",
            lsp_method: Some("textDocument/references"),
            file_id: Some(lib_file_id),
            range: Some(range(11, 8, 11, 14)),
            raw_json: Some(json!({ "source": "reference-evidence" })),
        })
        .await?;
    writer
        .insert_edge_evidence(EdgeEvidenceInput {
            edge_id: &ids.edge_run_helper_call,
            run_id,
            provider: "rust-analyzer",
            lsp_method: Some("callHierarchy/outgoingCalls"),
            file_id: Some(lib_file_id),
            range: Some(range(11, 8, 11, 14)),
            raw_json: Some(json!({ "source": "call-evidence" })),
        })
        .await?;
    writer.finish_run(run_id, "complete").await?;
    Ok(())
}

struct FixtureIds {
    file_src_lib: String,
    module_root: String,
    symbol_run: String,
    symbol_helper: String,
    edge_file_run: String,
    edge_run_helper_reference: String,
    edge_run_helper_call: String,
}

impl FixtureIds {
    fn new() -> Self {
        let workspace_id = 1;
        let file_src_lib = node_id(workspace_id, "rust", "file:///fixture/src/lib.rs");
        let module_root = node_id(workspace_id, "rust", "module:crate");
        let symbol_run = node_id(workspace_id, "rust", "function:crate::run");
        let symbol_helper = node_id(workspace_id, "rust", "function:crate::z_helper");
        Self {
            edge_file_run: edge_id(
                workspace_id,
                &file_src_lib,
                &symbol_run,
                "contains",
                Some("document-symbol"),
            ),
            edge_run_helper_reference: edge_id(
                workspace_id,
                &symbol_run,
                &symbol_helper,
                "references",
                Some("symbol"),
            ),
            edge_run_helper_call: edge_id(
                workspace_id,
                &symbol_run,
                &symbol_helper,
                "calls",
                Some("direct"),
            ),
            file_src_lib,
            module_root,
            symbol_run,
            symbol_helper,
        }
    }
}

fn range(start_line: i64, start_col: i64, end_line: i64, end_col: i64) -> TextRange {
    TextRange {
        start_line,
        start_col,
        end_line,
        end_col,
    }
}

fn temp_database_path() -> Result<PathBuf, Box<dyn Error>> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let index = TEMP_DATABASE_COUNTER.fetch_add(1, Ordering::Relaxed);
    Ok(std::env::temp_dir().join(format!(
        "semantic-graph-visualizer-server-{}-{timestamp}-{index}.db",
        std::process::id()
    )))
}
