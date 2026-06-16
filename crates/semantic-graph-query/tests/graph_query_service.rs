use semantic_graph_config::{QueryServiceConfig, QueryServiceConfigValues};
use semantic_graph_db_manager::{
    CloseStaleFileInput, EdgeEvidenceInput, EdgeInput, FileInput, NodeInput, OccurrenceInput,
    RouteObservationInput, RouteStatusCompleteInput, RouteStatusStartInput, TextRange, WriteHandle,
    WriteManager, edge_id, node_id,
};
use semantic_graph_query::{
    EdgeDetailsRequest, FileSummaryRequest, GraphQueryService, NeighborDirection, NeighborsRequest,
    NodeDetailsRequest, NodeSearchRequest, ProjectionRequest, QueryError, RouteStatusRequest,
    ShortestPathRequest,
};
use serde_json::{Value, json};
use std::{
    error::Error,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static TEMP_DATABASE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[tokio::test]
async fn stats_returns_counts_and_latest_runs() -> Result<(), Box<dyn Error>> {
    let fixture = seeded_database().await?;
    let service = GraphQueryService::new(fixture.database_path.clone());

    let stats = service.stats().await?;

    assert_eq!(1, stats.workspace_count);
    assert_eq!(3, stats.file_count);
    assert_eq!(9, stats.active_node_count);
    assert_eq!(2, stats.stale_node_count);
    assert_eq!(11, stats.active_edge_count);
    assert_eq!(2, stats.stale_edge_count);
    assert_eq!(5, stats.occurrence_count);
    assert_eq!(5, stats.edge_evidence_count);
    assert_eq!(2, stats.route_status_count);
    assert_eq!(2, stats.latest_runs.len());
    assert_eq!(fixture.stale_run_id, stats.latest_runs[0].run_id);
    assert_eq!("complete", stats.latest_runs[0].status);

    remove_database(fixture.database_path)?;
    Ok(())
}

#[tokio::test]
async fn query_service_uses_configured_non_default_limits() -> Result<(), Box<dyn Error>> {
    let fixture = seeded_database().await?;
    let service = GraphQueryService::with_query_service_config(
        fixture.database_path.clone(),
        query_service_config_with_latest_run_and_search_limits(1, 60)?,
    );

    let stats = service.stats().await?;
    assert_eq!(1, stats.latest_runs.len());
    assert_eq!(fixture.stale_run_id, stats.latest_runs[0].run_id);

    let allowed = service
        .search_nodes(NodeSearchRequest {
            query: "run".to_string(),
            limit: Some(60),
        })
        .await?;
    assert_eq!(60, allowed.applied_limit);

    assert_invalid_params(
        service
            .search_nodes(NodeSearchRequest {
                query: "run".to_string(),
                limit: Some(61),
            })
            .await,
        "limit must be between 1 and 60",
    )?;

    remove_database(fixture.database_path)?;
    Ok(())
}

#[tokio::test]
async fn search_nodes_finds_supported_fields_and_escapes_like_patterns()
-> Result<(), Box<dyn Error>> {
    let fixture = seeded_database().await?;
    let service = GraphQueryService::new(fixture.database_path.clone());

    let by_name = service
        .search_nodes(NodeSearchRequest {
            query: "run".to_string(),
            limit: Some(25),
        })
        .await?;
    assert_has_node(&by_name.results, &fixture.ids.symbol_run);

    let by_qualified_name = service
        .search_nodes(NodeSearchRequest {
            query: "crate::run".to_string(),
            limit: Some(25),
        })
        .await?;
    assert_has_node(&by_qualified_name.results, &fixture.ids.symbol_run);

    let by_display_name = service
        .search_nodes(NodeSearchRequest {
            query: "helper display".to_string(),
            limit: Some(25),
        })
        .await?;
    assert_has_node(&by_display_name.results, &fixture.ids.symbol_helper);

    let by_file_path = service
        .search_nodes(NodeSearchRequest {
            query: "z.rs".to_string(),
            limit: Some(25),
        })
        .await?;
    assert_has_node(&by_file_path.results, &fixture.ids.symbol_helper);

    let escaped_percent = service
        .search_nodes(NodeSearchRequest {
            query: "calc%_value".to_string(),
            limit: Some(25),
        })
        .await?;
    assert_eq!(1, escaped_percent.results.len());
    assert_eq!(
        fixture.ids.symbol_percent,
        escaped_percent.results[0].node_id
    );

    let escaped_backslash = service
        .search_nodes(NodeSearchRequest {
            query: r"slash\symbol".to_string(),
            limit: Some(25),
        })
        .await?;
    assert_eq!(1, escaped_backslash.results.len());
    assert_eq!(
        fixture.ids.symbol_backslash,
        escaped_backslash.results[0].node_id
    );

    let stale = service
        .search_nodes(NodeSearchRequest {
            query: "obsolete".to_string(),
            limit: Some(25),
        })
        .await?;
    assert!(stale.results.is_empty());

    let limited = service
        .search_nodes(NodeSearchRequest {
            query: "r".to_string(),
            limit: Some(1),
        })
        .await?;
    assert_eq!(1, limited.results.len());

    remove_database(fixture.database_path)?;
    Ok(())
}

#[tokio::test]
async fn details_return_metadata_evidence_and_exact_id_stale_state() -> Result<(), Box<dyn Error>> {
    let fixture = seeded_database().await?;
    let service = GraphQueryService::new(fixture.database_path.clone());

    let node = service
        .node_details(NodeDetailsRequest {
            node_id: fixture.ids.symbol_run.clone(),
        })
        .await?;

    assert_eq!(fixture.ids.symbol_run, node.node_id);
    assert_eq!("function", node.kind);
    assert_eq!("run", node.display_label);
    assert_eq!(Some("crate::run"), node.qualified_name.as_deref());
    assert_eq!(Some("src/lib.rs"), node.source_file_path.as_deref());
    assert_eq!(
        Some(fixture.ids.module_root.as_str()),
        node.container_node_id()
    );
    assert_eq!(json!({ "visibility": "public" }), node.properties_json);
    assert_eq!(2, node.incoming_edge_count);
    assert_eq!(3, node.outgoing_edge_count);
    assert!(node.valid_to_run_id.is_none());
    assert!(node.relations.iter().any(|relation| {
        relation.direction == "incoming"
            && relation.relation == "contains"
            && relation.edge_count == 2
    }));
    assert!(node.relations.iter().any(|relation| {
        relation.direction == "outgoing"
            && relation.relation == "references"
            && relation.edge_count == 1
    }));
    assert_eq!(1, node.occurrences.len());
    assert_eq!("definition", node.occurrences[0].role);
    assert_eq!("src/lib.rs", node.occurrences[0].source_file_path);
    assert_eq!(
        Some(json!({ "source": "occurrence" })),
        node.occurrences[0].raw_json
    );

    let edge = service
        .edge_details(EdgeDetailsRequest {
            edge_id: fixture.ids.edge_run_helper_reference.clone(),
        })
        .await?;
    assert_eq!(fixture.ids.edge_run_helper_reference, edge.edge_id);
    assert_eq!("references", edge.relation);
    assert_eq!(Some("symbol"), edge.context.as_deref());
    assert_eq!("EXTRACTED", edge.confidence);
    assert_eq!(2.0, edge.weight);
    assert_eq!(fixture.ids.symbol_run, edge.source.node_id);
    assert_eq!(fixture.ids.symbol_helper, edge.target.node_id);
    assert!(edge.valid_to_run_id.is_none());
    assert_eq!(
        json!({ "source_resolution": "symbol" }),
        edge.properties_json
    );
    assert_eq!(1, edge.evidence.len());
    assert_eq!("rust-analyzer", edge.evidence[0].provider);
    assert_eq!(
        Some("textDocument/references"),
        edge.evidence[0].lsp_method.as_deref()
    );
    assert_eq!(
        Some(json!({ "source": "reference-evidence" })),
        edge.evidence[0].raw_json
    );

    let stale_node = service
        .node_details(NodeDetailsRequest {
            node_id: fixture.ids.symbol_old.clone(),
        })
        .await?;
    assert_eq!(Some(fixture.stale_run_id), stale_node.valid_to_run_id);

    let stale_edge = service
        .edge_details(EdgeDetailsRequest {
            edge_id: fixture.ids.edge_run_old_reference.clone(),
        })
        .await?;
    assert_eq!(Some(fixture.stale_run_id), stale_edge.valid_to_run_id);

    remove_database(fixture.database_path)?;
    Ok(())
}

#[tokio::test]
async fn projection_neighbors_and_shortest_path_use_active_graph() -> Result<(), Box<dyn Error>> {
    let fixture = seeded_database().await?;
    let service = GraphQueryService::new(fixture.database_path.clone());

    let projection = service
        .projection(ProjectionRequest { limit: Some(10) })
        .await?;
    assert!(
        projection
            .nodes
            .iter()
            .any(|node| node.node_id == fixture.ids.file_src_lib)
    );
    assert_has_node(&projection.nodes, &fixture.ids.symbol_run);
    assert_has_node(&projection.nodes, &fixture.ids.symbol_helper);
    assert!(
        !projection
            .nodes
            .iter()
            .any(|node| node.node_id == fixture.ids.symbol_old)
    );
    assert!(
        projection
            .edges
            .iter()
            .any(|edge| edge.edge_id == fixture.ids.edge_run_helper_call)
    );
    assert!(
        !projection
            .edges
            .iter()
            .any(|edge| edge.edge_id == fixture.ids.edge_run_old_reference)
    );
    assert_eq!(Some(10), projection.metadata.requested_limit);
    assert_eq!(10, projection.metadata.applied_limit);

    let neighbors = service
        .neighbors(NeighborsRequest {
            node_id: fixture.ids.symbol_run.clone(),
            direction: Some(NeighborDirection::Both),
            relation: None,
            limit: Some(25),
        })
        .await?;
    assert_eq!(2, neighbors.incoming.len());
    assert_eq!(3, neighbors.outgoing.len());
    assert!(
        !neighbors
            .outgoing
            .iter()
            .any(|neighbor| neighbor.adjacent_node.node_id == fixture.ids.symbol_old)
    );

    let call_neighbors = service
        .neighbors(NeighborsRequest {
            node_id: fixture.ids.symbol_run.clone(),
            direction: Some(NeighborDirection::Outgoing),
            relation: Some("calls".to_string()),
            limit: Some(25),
        })
        .await?;
    assert!(call_neighbors.incoming.is_empty());
    assert_eq!(1, call_neighbors.outgoing.len());
    assert_eq!("calls", call_neighbors.outgoing[0].relation);

    let path = service
        .shortest_path(ShortestPathRequest {
            source_node_id: fixture.ids.module_root.clone(),
            target_node_id: fixture.ids.symbol_helper.clone(),
            max_depth: Some(4),
            max_visited_nodes: Some(50),
        })
        .await?;
    assert!(path.found);
    assert_eq!(3, path.nodes.len());
    assert_eq!(2, path.steps.len());
    assert_eq!(fixture.ids.module_root, path.nodes[0].node_id);
    assert_eq!(fixture.ids.symbol_helper, path.nodes[2].node_id);

    let missing_path = service
        .shortest_path(ShortestPathRequest {
            source_node_id: fixture.ids.symbol_run.clone(),
            target_node_id: fixture.ids.symbol_lonely.clone(),
            max_depth: Some(4),
            max_visited_nodes: Some(50),
        })
        .await?;
    assert!(!missing_path.found);
    assert!(missing_path.nodes.is_empty());

    remove_database(fixture.database_path)?;
    Ok(())
}

#[tokio::test]
async fn file_summary_returns_symbols_touching_edges_and_route_status() -> Result<(), Box<dyn Error>>
{
    let fixture = seeded_database().await?;
    let service = GraphQueryService::new(fixture.database_path.clone());

    let summary = service
        .file_summary(FileSummaryRequest {
            workspace_id: Some(fixture.workspace_id),
            root_uri: None,
            file_path: "src/lib.rs".to_string(),
            edge_limit: Some(25),
        })
        .await?;

    assert_eq!(fixture.workspace_id, summary.workspace_id);
    assert_eq!("file:///fixture", summary.root_uri);
    assert_eq!("src/lib.rs", summary.file.path);
    assert_eq!(
        Some(fixture.ids.file_src_lib.as_str()),
        summary.file_node.as_ref().map(|node| node.node_id.as_str())
    );
    assert_has_node(&summary.symbols, &fixture.ids.module_root);
    assert_has_node(&summary.symbols, &fixture.ids.symbol_run);
    assert!(
        summary
            .touching_edges
            .iter()
            .any(|edge| edge.edge_id == fixture.ids.edge_run_helper_call)
    );
    assert!(
        !summary
            .touching_edges
            .iter()
            .any(|edge| edge.edge_id == fixture.ids.edge_run_old_reference)
    );
    assert_eq!(1, summary.file_route_statuses.len());
    assert_eq!(
        "rust-file/document-symbols",
        summary.file_route_statuses[0].route
    );
    assert_eq!(
        json!({ "file": "clean" }),
        summary.file_route_statuses[0].diagnostics_json
    );
    assert_eq!(1, summary.workspace_route_statuses.len());
    assert_eq!(
        "rust-workspace/references",
        summary.workspace_route_statuses[0].route
    );

    remove_database(fixture.database_path)?;
    Ok(())
}

#[tokio::test]
async fn route_status_filters_and_parses_diagnostics() -> Result<(), Box<dyn Error>> {
    let fixture = seeded_database().await?;
    let service = GraphQueryService::new(fixture.database_path.clone());

    let by_file = service
        .route_status(RouteStatusRequest {
            workspace_id: None,
            root_uri: Some("file:///fixture".to_string()),
            route: None,
            scope: None,
            scope_key: None,
            file_path: Some("src/lib.rs".to_string()),
            limit: Some(25),
        })
        .await?;
    assert_eq!(1, by_file.statuses.len());
    assert_eq!("file", by_file.statuses[0].scope);
    assert_eq!(Some("src/lib.rs"), by_file.statuses[0].file_path.as_deref());
    assert_eq!(
        json!({ "file": "clean" }),
        by_file.statuses[0].diagnostics_json
    );

    let by_workspace_scope = service
        .route_status(RouteStatusRequest {
            workspace_id: Some(fixture.workspace_id),
            root_uri: None,
            route: None,
            scope: Some("workspace".to_string()),
            scope_key: Some("file:///fixture".to_string()),
            file_path: None,
            limit: Some(25),
        })
        .await?;
    assert_eq!(1, by_workspace_scope.statuses.len());
    assert_eq!(
        "rust-workspace/references",
        by_workspace_scope.statuses[0].route
    );
    assert_eq!(
        json!({ "workspace": "clean" }),
        by_workspace_scope.statuses[0].diagnostics_json
    );

    let limited = service
        .route_status(RouteStatusRequest {
            workspace_id: Some(fixture.workspace_id),
            root_uri: None,
            route: None,
            scope: None,
            scope_key: None,
            file_path: None,
            limit: Some(1),
        })
        .await?;
    assert_eq!(1, limited.statuses.len());
    assert_eq!(Some(1), limited.requested_limit);
    assert_eq!(1, limited.applied_limit);

    remove_database(fixture.database_path)?;
    Ok(())
}

#[tokio::test]
async fn validation_rejects_blank_inputs_and_excessive_limits() -> Result<(), Box<dyn Error>> {
    let fixture = seeded_database().await?;
    let service = GraphQueryService::new(fixture.database_path.clone());

    assert_invalid_params(
        service
            .search_nodes(NodeSearchRequest {
                query: " ".to_string(),
                limit: None,
            })
            .await,
        "query must not be blank",
    )?;
    assert_invalid_params(
        service
            .search_nodes(NodeSearchRequest {
                query: "run".to_string(),
                limit: Some(51),
            })
            .await,
        "limit must be between 1 and 50",
    )?;
    assert_invalid_params(
        service
            .node_details(NodeDetailsRequest {
                node_id: " ".to_string(),
            })
            .await,
        "nodeId must not be blank",
    )?;
    assert_invalid_params(
        service
            .edge_details(EdgeDetailsRequest {
                edge_id: " ".to_string(),
            })
            .await,
        "edgeId must not be blank",
    )?;
    assert_invalid_params(
        service
            .neighbors(NeighborsRequest {
                node_id: fixture.ids.symbol_run.clone(),
                direction: None,
                relation: None,
                limit: Some(101),
            })
            .await,
        "limit must be between 1 and 100",
    )?;
    assert_invalid_params(
        service
            .shortest_path(ShortestPathRequest {
                source_node_id: fixture.ids.symbol_run.clone(),
                target_node_id: fixture.ids.symbol_helper.clone(),
                max_depth: Some(13),
                max_visited_nodes: None,
            })
            .await,
        "maxDepth must be between 1 and 12",
    )?;
    assert_invalid_params(
        service
            .file_summary(FileSummaryRequest {
                workspace_id: None,
                root_uri: None,
                file_path: " ".to_string(),
                edge_limit: None,
            })
            .await,
        "filePath must not be blank",
    )?;
    assert_invalid_params(
        service
            .route_status(RouteStatusRequest {
                workspace_id: Some(0),
                root_uri: None,
                route: None,
                scope: None,
                scope_key: None,
                file_path: None,
                limit: None,
            })
            .await,
        "workspaceId must be positive",
    )?;

    let missing_node = service
        .node_details(NodeDetailsRequest {
            node_id: "missing-node".to_string(),
        })
        .await;
    assert_not_found(missing_node, "node 'missing-node' not found")?;

    remove_database(fixture.database_path)?;
    Ok(())
}

trait NodeDetailsContainer {
    fn container_node_id(&self) -> Option<&str>;
}

impl NodeDetailsContainer for semantic_graph_query::NodeDetails {
    fn container_node_id(&self) -> Option<&str> {
        self.container.as_ref().map(|node| node.node_id.as_str())
    }
}

fn assert_has_node<T>(nodes: &[T], expected_node_id: &str)
where
    T: HasNodeId,
{
    assert!(
        nodes.iter().any(|node| node.node_id() == expected_node_id),
        "expected node {expected_node_id}"
    );
}

trait HasNodeId {
    fn node_id(&self) -> &str;
}

impl HasNodeId for semantic_graph_query::NodeSearchResult {
    fn node_id(&self) -> &str {
        &self.node_id
    }
}

impl HasNodeId for semantic_graph_query::NodeSummary {
    fn node_id(&self) -> &str {
        &self.node_id
    }
}

fn assert_invalid_params<T>(
    result: Result<T, QueryError>,
    expected_message: &str,
) -> Result<(), Box<dyn Error>>
where
    T: std::fmt::Debug,
{
    match result {
        Err(QueryError::InvalidParams { message, .. }) => {
            assert_eq!(expected_message, message);
            Ok(())
        }
        other => Err(format!("expected invalid params error, got {other:?}").into()),
    }
}

fn assert_not_found<T>(
    result: Result<T, QueryError>,
    expected_message: &str,
) -> Result<(), Box<dyn Error>>
where
    T: std::fmt::Debug,
{
    match result {
        Err(QueryError::NotFound { message, .. }) => {
            assert_eq!(expected_message, message);
            Ok(())
        }
        other => Err(format!("expected not found error, got {other:?}").into()),
    }
}

async fn seeded_database() -> Result<Fixture, Box<dyn Error>> {
    let database_path = temp_database_path()?;
    let writer = WriteManager::start(&database_path).await?;
    writer.migrate().await?;
    let fixture = seed_fixture_database(&writer, database_path).await?;
    writer.shutdown().await?;
    Ok(fixture)
}

async fn seed_fixture_database(
    writer: &WriteHandle,
    database_path: PathBuf,
) -> Result<Fixture, Box<dyn Error>> {
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
            content_hash: Some("lib-hash"),
            last_seen_run_id: Some(run_id),
            properties_json: json!({ "fixture": "lib" }),
        })
        .await?;
    let helper_file_id = writer
        .upsert_file(FileInput {
            workspace_id,
            uri: "file:///fixture/src/z.rs",
            path: "src/z.rs",
            language: "rust",
            content_hash: Some("z-hash"),
            last_seen_run_id: Some(run_id),
            properties_json: json!({}),
        })
        .await?;
    let old_file_id = writer
        .upsert_file(FileInput {
            workspace_id,
            uri: "file:///fixture/src/old.rs",
            path: "src/old.rs",
            language: "rust",
            content_hash: Some("old-hash"),
            last_seen_run_id: Some(run_id),
            properties_json: json!({}),
        })
        .await?;
    let ids = FixtureIds::new(workspace_id);

    seed_nodes(
        writer,
        workspace_id,
        run_id,
        lib_file_id,
        helper_file_id,
        old_file_id,
        &ids,
    )
    .await?;
    seed_edges(writer, workspace_id, run_id, &ids).await?;
    seed_occurrences(writer, run_id, lib_file_id, helper_file_id, &ids).await?;
    seed_edge_evidence(writer, run_id, lib_file_id, old_file_id, &ids).await?;
    seed_route_statuses(writer, workspace_id, run_id, lib_file_id).await?;
    seed_route_observations(writer, workspace_id, run_id, lib_file_id, &ids).await?;
    writer.finish_run(run_id, "complete").await?;

    let stale_run_id = writer
        .start_run(workspace_id, "rust-analyzer", Some("fixture"), None)
        .await?;
    writer
        .close_stale_file(CloseStaleFileInput {
            workspace_id,
            run_id: stale_run_id,
            file_uri: "file:///fixture/src/old.rs",
        })
        .await?;
    writer.finish_run(stale_run_id, "complete").await?;

    Ok(Fixture {
        database_path,
        workspace_id,
        stale_run_id,
        ids,
    })
}

async fn seed_nodes(
    writer: &WriteHandle,
    workspace_id: i64,
    run_id: i64,
    lib_file_id: i64,
    helper_file_id: i64,
    old_file_id: i64,
    ids: &FixtureIds,
) -> Result<(), Box<dyn Error>> {
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
            kind: "file",
            name: "z.rs",
            qualified_name: Some("src/z.rs"),
            display_name: Some("z.rs"),
            symbol_key: "file:///fixture/src/z.rs",
            file_id: Some(helper_file_id),
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
            kind: "file",
            name: "old.rs",
            qualified_name: Some("src/old.rs"),
            display_name: Some("old.rs"),
            symbol_key: "file:///fixture/src/old.rs",
            file_id: Some(old_file_id),
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
            display_name: Some("helper display"),
            symbol_key: "function:crate::z_helper",
            file_id: Some(helper_file_id),
            range: Some(range(2, 0, 4, 1)),
            selection_range: Some(range(2, 3, 2, 9)),
            container_node_id: None,
            properties_json: json!({}),
            run_id: Some(run_id),
        })
        .await?;
    seed_extra_symbol_nodes(
        writer,
        workspace_id,
        run_id,
        helper_file_id,
        old_file_id,
        ids,
    )
    .await?;
    Ok(())
}

async fn seed_extra_symbol_nodes(
    writer: &WriteHandle,
    workspace_id: i64,
    run_id: i64,
    helper_file_id: i64,
    old_file_id: i64,
    ids: &FixtureIds,
) -> Result<(), Box<dyn Error>> {
    writer
        .upsert_node(NodeInput {
            workspace_id,
            language: "rust",
            kind: "function",
            name: "lonely",
            qualified_name: Some("crate::lonely"),
            display_name: Some("lonely"),
            symbol_key: "function:crate::lonely",
            file_id: Some(helper_file_id),
            range: Some(range(20, 0, 22, 1)),
            selection_range: Some(range(20, 3, 20, 9)),
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
            name: "calc%_value",
            qualified_name: Some("crate::calc%_value"),
            display_name: Some("percent symbol"),
            symbol_key: "function:crate::calc%_value",
            file_id: Some(helper_file_id),
            range: Some(range(30, 0, 32, 1)),
            selection_range: Some(range(30, 3, 30, 14)),
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
            name: r"slash\symbol",
            qualified_name: Some(r"crate::slash\symbol"),
            display_name: Some("slash symbol"),
            symbol_key: r"function:crate::slash\symbol",
            file_id: Some(helper_file_id),
            range: Some(range(40, 0, 42, 1)),
            selection_range: Some(range(40, 3, 40, 15)),
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
            name: "calcAAvalue",
            qualified_name: Some("crate::calcAAvalue"),
            display_name: Some("calc decoy"),
            symbol_key: "function:crate::calcAAvalue",
            file_id: Some(helper_file_id),
            range: Some(range(50, 0, 52, 1)),
            selection_range: Some(range(50, 3, 50, 14)),
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
            name: "obsolete",
            qualified_name: Some("crate::obsolete"),
            display_name: Some("obsolete"),
            symbol_key: "function:crate::obsolete",
            file_id: Some(old_file_id),
            range: Some(range(2, 0, 3, 1)),
            selection_range: Some(range(2, 3, 2, 11)),
            container_node_id: None,
            properties_json: json!({ "state": "old" }),
            run_id: Some(run_id),
        })
        .await?;

    assert_eq!(
        ids.symbol_percent,
        node_id(workspace_id, "rust", "function:crate::calc%_value")
    );
    Ok(())
}

async fn seed_edges(
    writer: &WriteHandle,
    workspace_id: i64,
    run_id: i64,
    ids: &FixtureIds,
) -> Result<(), Box<dyn Error>> {
    upsert_edge(
        writer,
        EdgeFixtureInput {
            workspace_id,
            run_id,
            source_node_id: &ids.file_src_lib,
            target_node_id: &ids.module_root,
            relation: "contains",
            context: Some("document-symbol"),
            confidence_score: 1.0,
            weight: 1.0,
            properties_json: json!({}),
        },
    )
    .await?;
    upsert_edge(
        writer,
        EdgeFixtureInput {
            workspace_id,
            run_id,
            source_node_id: &ids.file_src_lib,
            target_node_id: &ids.symbol_run,
            relation: "contains",
            context: Some("document-symbol"),
            confidence_score: 1.0,
            weight: 1.0,
            properties_json: json!({ "source": "documentSymbol" }),
        },
    )
    .await?;
    upsert_edge(
        writer,
        EdgeFixtureInput {
            workspace_id,
            run_id,
            source_node_id: &ids.module_root,
            target_node_id: &ids.symbol_run,
            relation: "contains",
            context: Some("document-symbol"),
            confidence_score: 1.0,
            weight: 1.0,
            properties_json: json!({}),
        },
    )
    .await?;
    upsert_edge(
        writer,
        EdgeFixtureInput {
            workspace_id,
            run_id,
            source_node_id: &ids.file_src_z,
            target_node_id: &ids.symbol_helper,
            relation: "contains",
            context: Some("document-symbol"),
            confidence_score: 1.0,
            weight: 1.0,
            properties_json: json!({}),
        },
    )
    .await?;
    upsert_edge(
        writer,
        EdgeFixtureInput {
            workspace_id,
            run_id,
            source_node_id: &ids.symbol_run,
            target_node_id: &ids.symbol_helper,
            relation: "contains",
            context: Some("document-symbol"),
            confidence_score: 1.0,
            weight: 1.0,
            properties_json: json!({}),
        },
    )
    .await?;
    upsert_edge(
        writer,
        EdgeFixtureInput {
            workspace_id,
            run_id,
            source_node_id: &ids.symbol_run,
            target_node_id: &ids.symbol_helper,
            relation: "references",
            context: Some("symbol"),
            confidence_score: 1.0,
            weight: 2.0,
            properties_json: json!({ "source_resolution": "symbol" }),
        },
    )
    .await?;
    upsert_edge(
        writer,
        EdgeFixtureInput {
            workspace_id,
            run_id,
            source_node_id: &ids.symbol_run,
            target_node_id: &ids.symbol_helper,
            relation: "calls",
            context: Some("direct"),
            confidence_score: 1.0,
            weight: 2.0,
            properties_json: json!({ "source_resolution": "symbol" }),
        },
    )
    .await?;
    upsert_edge(
        writer,
        EdgeFixtureInput {
            workspace_id,
            run_id,
            source_node_id: &ids.file_src_z,
            target_node_id: &ids.symbol_lonely,
            relation: "contains",
            context: Some("document-symbol"),
            confidence_score: 1.0,
            weight: 1.0,
            properties_json: json!({}),
        },
    )
    .await?;
    upsert_edge(
        writer,
        EdgeFixtureInput {
            workspace_id,
            run_id,
            source_node_id: &ids.file_src_z,
            target_node_id: &ids.symbol_percent,
            relation: "contains",
            context: Some("document-symbol"),
            confidence_score: 1.0,
            weight: 1.0,
            properties_json: json!({}),
        },
    )
    .await?;
    upsert_edge(
        writer,
        EdgeFixtureInput {
            workspace_id,
            run_id,
            source_node_id: &ids.file_src_z,
            target_node_id: &ids.symbol_backslash,
            relation: "contains",
            context: Some("document-symbol"),
            confidence_score: 1.0,
            weight: 1.0,
            properties_json: json!({}),
        },
    )
    .await?;
    upsert_edge(
        writer,
        EdgeFixtureInput {
            workspace_id,
            run_id,
            source_node_id: &ids.file_src_z,
            target_node_id: &ids.symbol_decoy,
            relation: "contains",
            context: Some("document-symbol"),
            confidence_score: 1.0,
            weight: 1.0,
            properties_json: json!({}),
        },
    )
    .await?;
    upsert_edge(
        writer,
        EdgeFixtureInput {
            workspace_id,
            run_id,
            source_node_id: &ids.file_src_old,
            target_node_id: &ids.symbol_old,
            relation: "contains",
            context: Some("document-symbol"),
            confidence_score: 1.0,
            weight: 1.0,
            properties_json: json!({}),
        },
    )
    .await?;
    upsert_edge(
        writer,
        EdgeFixtureInput {
            workspace_id,
            run_id,
            source_node_id: &ids.symbol_run,
            target_node_id: &ids.symbol_old,
            relation: "references",
            context: Some("symbol"),
            confidence_score: 1.0,
            weight: 1.0,
            properties_json: json!({ "state": "old" }),
        },
    )
    .await?;
    Ok(())
}

struct EdgeFixtureInput<'a> {
    workspace_id: i64,
    run_id: i64,
    source_node_id: &'a str,
    target_node_id: &'a str,
    relation: &'a str,
    context: Option<&'a str>,
    confidence_score: f64,
    weight: f64,
    properties_json: Value,
}

async fn upsert_edge(
    writer: &WriteHandle,
    input: EdgeFixtureInput<'_>,
) -> Result<(), Box<dyn Error>> {
    writer
        .upsert_edge(EdgeInput {
            workspace_id: input.workspace_id,
            src_node_id: input.source_node_id,
            dst_node_id: input.target_node_id,
            relation: input.relation,
            context: input.context,
            confidence: "EXTRACTED",
            confidence_score: input.confidence_score,
            weight: input.weight,
            properties_json: input.properties_json,
            run_id: Some(input.run_id),
        })
        .await?;
    Ok(())
}

async fn seed_occurrences(
    writer: &WriteHandle,
    run_id: i64,
    lib_file_id: i64,
    helper_file_id: i64,
    ids: &FixtureIds,
) -> Result<(), Box<dyn Error>> {
    insert_occurrence(
        writer,
        OccurrenceFixtureInput {
            node_id: &ids.symbol_run,
            run_id,
            file_id: lib_file_id,
            role: "definition",
            range: range(10, 4, 12, 5),
            enclosing_node_id: Some(&ids.module_root),
            raw_json: json!({ "source": "occurrence" }),
        },
    )
    .await?;
    insert_occurrence(
        writer,
        OccurrenceFixtureInput {
            node_id: &ids.symbol_helper,
            run_id,
            file_id: lib_file_id,
            role: "reference",
            range: range(11, 8, 11, 14),
            enclosing_node_id: Some(&ids.symbol_run),
            raw_json: json!({ "source": "reference-occurrence" }),
        },
    )
    .await?;
    insert_occurrence(
        writer,
        OccurrenceFixtureInput {
            node_id: &ids.symbol_helper,
            run_id,
            file_id: lib_file_id,
            role: "call",
            range: range(11, 8, 11, 14),
            enclosing_node_id: Some(&ids.symbol_run),
            raw_json: json!({ "source": "call-occurrence" }),
        },
    )
    .await?;
    insert_occurrence(
        writer,
        OccurrenceFixtureInput {
            node_id: &ids.symbol_percent,
            run_id,
            file_id: helper_file_id,
            role: "definition",
            range: range(30, 0, 32, 1),
            enclosing_node_id: None,
            raw_json: json!({ "source": "percent-occurrence" }),
        },
    )
    .await?;
    insert_occurrence(
        writer,
        OccurrenceFixtureInput {
            node_id: &ids.symbol_backslash,
            run_id,
            file_id: helper_file_id,
            role: "definition",
            range: range(40, 0, 42, 1),
            enclosing_node_id: None,
            raw_json: json!({ "source": "backslash-occurrence" }),
        },
    )
    .await?;
    Ok(())
}

struct OccurrenceFixtureInput<'a> {
    node_id: &'a str,
    run_id: i64,
    file_id: i64,
    role: &'a str,
    range: TextRange,
    enclosing_node_id: Option<&'a str>,
    raw_json: Value,
}

async fn insert_occurrence(
    writer: &WriteHandle,
    input: OccurrenceFixtureInput<'_>,
) -> Result<(), Box<dyn Error>> {
    writer
        .insert_occurrence(OccurrenceInput {
            node_id: input.node_id,
            run_id: input.run_id,
            file_id: input.file_id,
            role: input.role,
            range: input.range,
            enclosing_node_id: input.enclosing_node_id,
            raw_json: Some(input.raw_json),
        })
        .await?;
    Ok(())
}

async fn seed_edge_evidence(
    writer: &WriteHandle,
    run_id: i64,
    lib_file_id: i64,
    old_file_id: i64,
    ids: &FixtureIds,
) -> Result<(), Box<dyn Error>> {
    insert_edge_evidence(
        writer,
        &ids.edge_file_run,
        run_id,
        lib_file_id,
        Some("textDocument/documentSymbol"),
        range(10, 4, 12, 5),
        json!({ "source": "edge-evidence" }),
    )
    .await?;
    insert_edge_evidence(
        writer,
        &ids.edge_run_helper_reference,
        run_id,
        lib_file_id,
        Some("textDocument/references"),
        range(11, 8, 11, 14),
        json!({ "source": "reference-evidence" }),
    )
    .await?;
    insert_edge_evidence(
        writer,
        &ids.edge_run_helper_call,
        run_id,
        lib_file_id,
        Some("callHierarchy/outgoingCalls"),
        range(11, 8, 11, 14),
        json!({ "source": "call-evidence" }),
    )
    .await?;
    insert_edge_evidence(
        writer,
        &ids.edge_file_old,
        run_id,
        old_file_id,
        Some("textDocument/documentSymbol"),
        range(2, 0, 3, 1),
        json!({ "source": "old-document-symbol" }),
    )
    .await?;
    insert_edge_evidence(
        writer,
        &ids.edge_run_old_reference,
        run_id,
        old_file_id,
        Some("textDocument/references"),
        range(2, 0, 3, 1),
        json!({ "source": "old-reference" }),
    )
    .await?;
    Ok(())
}

async fn insert_edge_evidence(
    writer: &WriteHandle,
    edge_id: &str,
    run_id: i64,
    file_id: i64,
    lsp_method: Option<&str>,
    range: TextRange,
    raw_json: Value,
) -> Result<(), Box<dyn Error>> {
    writer
        .insert_edge_evidence(EdgeEvidenceInput {
            edge_id,
            run_id,
            provider: "rust-analyzer",
            lsp_method,
            file_id: Some(file_id),
            range: Some(range),
            raw_json: Some(raw_json),
        })
        .await?;
    Ok(())
}

async fn seed_route_statuses(
    writer: &WriteHandle,
    workspace_id: i64,
    run_id: i64,
    lib_file_id: i64,
) -> Result<(), Box<dyn Error>> {
    writer
        .start_route_status(RouteStatusStartInput {
            workspace_id,
            route: "rust-file/document-symbols",
            scope: "file",
            scope_key: "file:///fixture/src/lib.rs",
            file_id: Some(lib_file_id),
            provider: "rust-analyzer",
            provider_version: Some("fixture"),
            content_hash: Some("lib-hash"),
            run_id,
            diagnostics_json: json!({ "phase": "started" }),
        })
        .await?;
    writer
        .complete_route_status(RouteStatusCompleteInput {
            workspace_id,
            route: "rust-file/document-symbols",
            scope: "file",
            scope_key: "file:///fixture/src/lib.rs",
            provider: "rust-analyzer",
            provider_version: Some("fixture"),
            content_hash: Some("lib-hash"),
            run_id,
            diagnostics_json: json!({ "file": "clean" }),
        })
        .await?;
    writer
        .start_route_status(RouteStatusStartInput {
            workspace_id,
            route: "rust-workspace/references",
            scope: "workspace",
            scope_key: "file:///fixture",
            file_id: None,
            provider: "rust-analyzer",
            provider_version: Some("fixture"),
            content_hash: None,
            run_id,
            diagnostics_json: json!({ "phase": "started" }),
        })
        .await?;
    writer
        .complete_route_status(RouteStatusCompleteInput {
            workspace_id,
            route: "rust-workspace/references",
            scope: "workspace",
            scope_key: "file:///fixture",
            provider: "rust-analyzer",
            provider_version: Some("fixture"),
            content_hash: None,
            run_id,
            diagnostics_json: json!({ "workspace": "clean" }),
        })
        .await?;
    Ok(())
}

async fn seed_route_observations(
    writer: &WriteHandle,
    workspace_id: i64,
    run_id: i64,
    lib_file_id: i64,
    ids: &FixtureIds,
) -> Result<(), Box<dyn Error>> {
    writer
        .record_route_observation(RouteObservationInput {
            workspace_id,
            run_id,
            route: "rust-file/document-symbols",
            scope: "file",
            scope_key: "file:///fixture/src/lib.rs",
            provider: "rust-analyzer",
            entity_kind: "node",
            entity_id: &ids.symbol_run,
            source_file_id: Some(lib_file_id),
            properties_json: json!({ "observed": "node" }),
        })
        .await?;
    writer
        .record_route_observation(RouteObservationInput {
            workspace_id,
            run_id,
            route: "rust-file/document-symbols",
            scope: "file",
            scope_key: "file:///fixture/src/lib.rs",
            provider: "rust-analyzer",
            entity_kind: "edge",
            entity_id: &ids.edge_file_run,
            source_file_id: Some(lib_file_id),
            properties_json: json!({ "observed": "edge" }),
        })
        .await?;
    writer
        .record_route_observation(RouteObservationInput {
            workspace_id,
            run_id,
            route: "rust-workspace/references",
            scope: "workspace",
            scope_key: "file:///fixture",
            provider: "rust-analyzer",
            entity_kind: "edge",
            entity_id: &ids.edge_run_helper_reference,
            source_file_id: Some(lib_file_id),
            properties_json: json!({ "observed": "reference" }),
        })
        .await?;
    Ok(())
}

#[derive(Debug)]
struct Fixture {
    database_path: PathBuf,
    workspace_id: i64,
    stale_run_id: i64,
    ids: FixtureIds,
}

#[derive(Debug)]
struct FixtureIds {
    file_src_lib: String,
    file_src_z: String,
    file_src_old: String,
    module_root: String,
    symbol_run: String,
    symbol_helper: String,
    symbol_lonely: String,
    symbol_percent: String,
    symbol_backslash: String,
    symbol_decoy: String,
    symbol_old: String,
    edge_file_run: String,
    edge_run_helper_reference: String,
    edge_run_helper_call: String,
    edge_file_old: String,
    edge_run_old_reference: String,
}

impl FixtureIds {
    fn new(workspace_id: i64) -> Self {
        let file_src_lib = node_id(workspace_id, "rust", "file:///fixture/src/lib.rs");
        let file_src_z = node_id(workspace_id, "rust", "file:///fixture/src/z.rs");
        let file_src_old = node_id(workspace_id, "rust", "file:///fixture/src/old.rs");
        let module_root = node_id(workspace_id, "rust", "module:crate");
        let symbol_run = node_id(workspace_id, "rust", "function:crate::run");
        let symbol_helper = node_id(workspace_id, "rust", "function:crate::z_helper");
        let symbol_lonely = node_id(workspace_id, "rust", "function:crate::lonely");
        let symbol_percent = node_id(workspace_id, "rust", "function:crate::calc%_value");
        let symbol_backslash = node_id(workspace_id, "rust", r"function:crate::slash\symbol");
        let symbol_decoy = node_id(workspace_id, "rust", "function:crate::calcAAvalue");
        let symbol_old = node_id(workspace_id, "rust", "function:crate::obsolete");

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
            edge_file_old: edge_id(
                workspace_id,
                &file_src_old,
                &symbol_old,
                "contains",
                Some("document-symbol"),
            ),
            edge_run_old_reference: edge_id(
                workspace_id,
                &symbol_run,
                &symbol_old,
                "references",
                Some("symbol"),
            ),
            file_src_lib,
            file_src_z,
            file_src_old,
            module_root,
            symbol_run,
            symbol_helper,
            symbol_lonely,
            symbol_percent,
            symbol_backslash,
            symbol_decoy,
            symbol_old,
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
        "semantic-graph-query-{}-{timestamp}-{index}.db",
        std::process::id()
    )))
}

fn remove_database(database_path: PathBuf) -> Result<(), Box<dyn Error>> {
    std::fs::remove_file(database_path)?;
    Ok(())
}

fn query_service_config_with_latest_run_and_search_limits(
    latest_run_limit: i64,
    max_search_limit: i64,
) -> Result<QueryServiceConfig, Box<dyn Error>> {
    Ok(QueryServiceConfig::new(QueryServiceConfigValues {
        latest_run_limit,
        max_search_limit,
        max_projection_limit: 1000,
        max_neighbors_limit: 100,
        max_file_edge_limit: 200,
        max_route_status_limit: 200,
        max_shortest_path_depth: 12,
        max_shortest_path_visited: 5000,
    })?)
}
