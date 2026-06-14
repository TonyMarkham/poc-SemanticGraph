use crate::{
    dto::{
        GraphEdgeDetailsDto, GraphNodeDetailsDto, GraphNodeSearchResultsDto, JsonRpcResponseDto,
    },
    query::GraphQueryService,
    rpc::rpc_handler,
    server::AppState,
};

use axum::{Json, body::Bytes, extract::State};
use serde_json::{Value, json};
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use std::{
    error::Error,
    io,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

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
async fn node_details_returns_metadata_occurrences_and_counts() -> Result<(), Box<dyn Error>> {
    let database_path = seeded_database_path().await?;

    let service = GraphQueryService::new(database_path.clone());
    let details = service.node_details("symbol-run").await?;

    assert_eq!("symbol-run", details.node_id);
    assert_eq!("function", details.kind);
    assert_eq!("run", details.display_label);
    assert_eq!(Some("crate::run"), details.qualified_name.as_deref());
    assert_eq!(Some("src/lib.rs"), details.source_file_path.as_deref());
    assert_eq!(Some("module-root"), details.container_node_id.as_deref());
    assert_eq!(Some("crate"), details.container_display_label.as_deref());
    assert_eq!(2, details.incoming_edge_count);
    assert_eq!(2, details.outgoing_edge_count);
    assert_eq!(3, details.relations.len());
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

    let service = GraphQueryService::new(database_path.clone());
    let details = service.edge_details("edge-run-helper-reference").await?;

    assert_eq!("edge-run-helper-reference", details.edge_id);
    assert_eq!("references", details.relation);
    assert_eq!(Some("symbol"), details.context.as_deref());
    assert_eq!("EXTRACTED", details.confidence);
    assert_eq!(2.0, details.weight);
    assert_eq!("symbol-run", details.source.node_id);
    assert_eq!("symbol-helper", details.target.node_id);
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
async fn edge_details_returns_endpoints_and_evidence() -> Result<(), Box<dyn Error>> {
    let database_path = seeded_database_path().await?;

    let service = GraphQueryService::new(database_path.clone());
    let details = service.edge_details("edge-file-run").await?;

    assert_eq!("edge-file-run", details.edge_id);
    assert_eq!("contains", details.relation);
    assert_eq!("EXTRACTED", details.confidence);
    assert_eq!("file-src-lib", details.source.node_id);
    assert_eq!("lib.rs", details.source.display_label);
    assert_eq!("symbol-run", details.target.node_id);
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

    let service = GraphQueryService::new(database_path.clone());

    let by_name = service.search_nodes("run", 25).await?;
    assert!(
        by_name
            .results
            .iter()
            .any(|result| result.node_id == "symbol-run")
    );

    let by_qualified_name = service.search_nodes("crate::run", 25).await?;
    assert!(
        by_qualified_name
            .results
            .iter()
            .any(|result| result.node_id == "symbol-run")
    );

    let by_file_path = service.search_nodes("z.rs", 25).await?;
    assert!(
        by_file_path
            .results
            .iter()
            .any(|result| result.node_id == "symbol-helper")
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
            "params": { "nodeId": "symbol-run" }
        }),
    )
    .await?;
    let node_details: GraphNodeDetailsDto = result_value(node_details)?;
    assert_eq!("symbol-run", node_details.node_id);

    let edge_details = rpc_request(
        state.clone(),
        json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "graph.edge_details",
            "params": { "edgeId": "edge-file-run" }
        }),
    )
    .await?;
    let edge_details: GraphEdgeDetailsDto = result_value(edge_details)?;
    assert_eq!("edge-file-run", edge_details.edge_id);

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
            .any(|result| result.node_id == "symbol-run")
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
    let pool = create_fixture_database(&database_path).await?;
    seed_fixture_database(&pool).await?;
    pool.close().await;
    Ok(database_path)
}

async fn create_fixture_database(database_path: &PathBuf) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(database_path)
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;

    sqlx::raw_sql(include_str!(
        "../../../semantic-graph-store/migrations/01_create_graph_store.sql"
    ))
    .execute(&pool)
    .await?;

    Ok(pool)
}

async fn seed_fixture_database(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::raw_sql(
        r#"
        INSERT INTO workspaces (id, root_uri, kind)
        VALUES (1, 'file:///fixture', 'rust');

        INSERT INTO extraction_runs (id, workspace_id, provider, provider_version, status)
        VALUES (1, 1, 'rust-analyzer', 'fixture', 'complete');

        INSERT INTO files (id, workspace_id, uri, path, language, last_seen_run_id)
        VALUES
          (1, 1, 'file:///fixture/src/lib.rs', 'src/lib.rs', 'rust', 1),
          (2, 1, 'file:///fixture/src/z.rs', 'src/z.rs', 'rust', 1);

        INSERT INTO nodes (
          id,
          workspace_id,
          language,
          kind,
          name,
          qualified_name,
          display_name,
          symbol_key,
          file_id,
          start_line,
          start_col,
          end_line,
          end_col,
          selection_start_line,
          selection_start_col,
          container_node_id,
          properties_json,
          first_seen_run_id,
          last_seen_run_id,
          valid_to_run_id
        )
        VALUES
          (
            'file-src-lib',
            1,
            'rust',
            'file',
            'lib.rs',
            'src/lib.rs',
            'lib.rs',
            'file:///fixture/src/lib.rs',
            1,
            NULL,
            NULL,
            NULL,
            NULL,
            NULL,
            NULL,
            NULL,
            '{}',
            1,
            1,
            NULL
          ),
          (
            'module-root',
            1,
            'rust',
            'module',
            'crate',
            'crate',
            'crate',
            'module:crate',
            1,
            1,
            0,
            30,
            0,
            1,
            0,
            NULL,
            '{}',
            1,
            1,
            NULL
          ),
          (
            'symbol-run',
            1,
            'rust',
            'function',
            'run',
            'crate::run',
            'run',
            'function:crate::run',
            1,
            10,
            4,
            12,
            5,
            10,
            7,
            'module-root',
            '{"visibility":"public"}',
            1,
            1,
            NULL
          ),
          (
            'symbol-helper',
            1,
            'rust',
            'function',
            'helper',
            'crate::z_helper',
            'helper',
            'function:crate::z_helper',
            2,
            2,
            0,
            4,
            1,
            2,
            3,
            NULL,
            '{}',
            1,
            1,
            NULL
          );

        INSERT INTO edges (
          id,
          workspace_id,
          src_node_id,
          dst_node_id,
          relation,
          context,
          confidence,
          confidence_score,
          weight,
          properties_json,
          first_seen_run_id,
          last_seen_run_id,
          valid_to_run_id
        )
        VALUES
          (
            'edge-file-run',
            1,
            'file-src-lib',
            'symbol-run',
            'contains',
            'document-symbol',
            'EXTRACTED',
            1.0,
            1.0,
            '{"source":"documentSymbol"}',
            1,
            1,
            NULL
          ),
          (
            'edge-module-run',
            1,
            'module-root',
            'symbol-run',
            'contains',
            'document-symbol',
            'EXTRACTED',
            1.0,
            1.0,
            '{}',
            1,
            1,
            NULL
          ),
          (
            'edge-run-helper',
            1,
            'symbol-run',
            'symbol-helper',
            'contains',
            'document-symbol',
            'EXTRACTED',
            1.0,
            1.0,
            '{}',
            1,
            1,
            NULL
          ),
          (
            'edge-run-helper-reference',
            1,
            'symbol-run',
            'symbol-helper',
            'references',
            'symbol',
            'EXTRACTED',
            1.0,
            2.0,
            '{"source_resolution":"symbol"}',
            1,
            1,
            NULL
          );

        INSERT INTO occurrences (
          id,
          node_id,
          run_id,
          file_id,
          role,
          start_line,
          start_col,
          end_line,
          end_col,
          enclosing_node_id,
          raw_json
        )
        VALUES
          (
            1,
            'symbol-run',
            1,
            1,
            'definition',
            10,
            4,
            12,
            5,
            'module-root',
            '{"source":"occurrence"}'
          ),
          (
            2,
            'symbol-helper',
            1,
            1,
            'reference',
            11,
            8,
            11,
            14,
            'symbol-run',
            '{"source":"reference-occurrence"}'
          );

        INSERT INTO edge_evidence (
          id,
          edge_id,
          run_id,
          provider,
          lsp_method,
          file_id,
          start_line,
          start_col,
          end_line,
          end_col,
          raw_json
        )
        VALUES
          (
            1,
            'edge-file-run',
            1,
            'rust-analyzer',
            'textDocument/documentSymbol',
            1,
            10,
            4,
            12,
            5,
            '{"source":"edge-evidence"}'
          ),
          (
            2,
            'edge-run-helper-reference',
            1,
            'rust-analyzer',
            'textDocument/references',
            1,
            11,
            8,
            11,
            14,
            '{"source":"reference-evidence"}'
          );
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

fn temp_database_path() -> Result<PathBuf, Box<dyn Error>> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    Ok(std::env::temp_dir().join(format!("semantic-graph-visualizer-server-{timestamp}.db")))
}
