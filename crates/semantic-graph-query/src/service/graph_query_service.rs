use crate::{
    QueryError, QueryResult,
    model::{
        EdgeDetails, EdgeDetailsRequest, EdgeSummary, FileSummary, FileSummaryRequest, GraphPath,
        GraphPathStep, GraphProjection, GraphStats, NeighborDirection, NeighborsRequest,
        NodeDetails, NodeDetailsRequest, NodeNeighbor, NodeNeighbors, NodeSearchRequest,
        NodeSearchResults, NodeSummary, ProjectionMetadata, ProjectionRequest, RouteStatus,
        RouteStatusRequest, RouteStatusResults, ShortestPathRequest,
    },
    row::{
        EdgeDetailsRow, EdgeEndpointRow, EdgeEvidenceRow, EdgeSummaryRow, ExtractionRunSummaryRow,
        FileSummaryFileRow, NodeDetailsRow, NodeNeighborRow, NodeOccurrenceRow,
        NodeRelationSummaryRow, NodeSearchResultRow, NodeSummaryRow, RouteStatusRow,
    },
    service::route_status_filters::RouteStatusFilters,
    sqlite::{escape_like_pattern, open_read_only_pool},
};

use semantic_graph_config::QueryServiceConfig;

use sqlx::SqlitePool;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::PathBuf,
};

const DEFAULT_SEARCH_LIMIT: i64 = 25;
const DEFAULT_PROJECTION_LIMIT: i64 = 150;
const DEFAULT_NEIGHBORS_LIMIT: i64 = 25;
const DEFAULT_FILE_EDGE_LIMIT: i64 = 50;
const DEFAULT_ROUTE_STATUS_LIMIT: i64 = 50;
const DEFAULT_SHORTEST_PATH_DEPTH: i64 = 6;
const DEFAULT_SHORTEST_PATH_VISITED: i64 = 500;

#[derive(Debug, Clone)]
pub struct GraphQueryService {
    database_path: PathBuf,
    query_service_config: QueryServiceConfig,
}

impl GraphQueryService {
    pub fn new(database_path: PathBuf) -> Self {
        Self::with_query_service_config(database_path, QueryServiceConfig::default())
    }

    pub fn with_query_service_config(
        database_path: PathBuf,
        query_service_config: QueryServiceConfig,
    ) -> Self {
        Self {
            database_path,
            query_service_config,
        }
    }

    pub async fn stats(&self) -> QueryResult<GraphStats> {
        let pool = open_read_only_pool(&self.database_path).await?;

        let latest_runs =
            load_latest_runs(&pool, self.query_service_config.latest_run_limit()).await?;

        Ok(GraphStats {
            workspace_count: count(&pool, "SELECT COUNT(*) FROM workspaces").await?,
            file_count: count(&pool, "SELECT COUNT(*) FROM files").await?,
            active_node_count: count(
                &pool,
                "SELECT COUNT(*) FROM nodes WHERE valid_to_run_id IS NULL",
            )
            .await?,
            stale_node_count: count(
                &pool,
                "SELECT COUNT(*) FROM nodes WHERE valid_to_run_id IS NOT NULL",
            )
            .await?,
            active_edge_count: count(
                &pool,
                "SELECT COUNT(*) FROM edges WHERE valid_to_run_id IS NULL",
            )
            .await?,
            stale_edge_count: count(
                &pool,
                "SELECT COUNT(*) FROM edges WHERE valid_to_run_id IS NOT NULL",
            )
            .await?,
            occurrence_count: count(&pool, "SELECT COUNT(*) FROM occurrences").await?,
            edge_evidence_count: count(&pool, "SELECT COUNT(*) FROM edge_evidence").await?,
            route_status_count: count(&pool, "SELECT COUNT(*) FROM extraction_route_status")
                .await?,
            latest_runs,
        })
    }

    pub async fn search_nodes(&self, request: NodeSearchRequest) -> QueryResult<NodeSearchResults> {
        let query = required_text(request.query, "query")?;
        let limit = resolve_limit(
            request.limit,
            DEFAULT_SEARCH_LIMIT,
            self.query_service_config.max_search_limit(),
            "limit",
        )?;
        let pool = open_read_only_pool(&self.database_path).await?;
        let results = load_node_search_results(&pool, &query, limit).await?;

        Ok(NodeSearchResults {
            results,
            requested_limit: request.limit,
            applied_limit: limit,
        })
    }

    pub async fn node_details(&self, request: NodeDetailsRequest) -> QueryResult<NodeDetails> {
        let node_id = required_text(request.node_id, "nodeId")?;
        let pool = open_read_only_pool(&self.database_path).await?;
        let node = load_node_details_row(&pool, &node_id).await?;
        let relations = load_node_relation_summaries(&pool, &node_id).await?;
        let occurrences = load_node_occurrences(&pool, &node_id).await?;

        node.into_model(relations, occurrences)
    }

    pub async fn edge_details(&self, request: EdgeDetailsRequest) -> QueryResult<EdgeDetails> {
        let edge_id = required_text(request.edge_id, "edgeId")?;
        let pool = open_read_only_pool(&self.database_path).await?;
        let edge = load_edge_details_row(&pool, &edge_id).await?;
        let source_node_id = edge.source_node_id.clone();
        let target_node_id = edge.target_node_id.clone();
        let source = load_edge_endpoint(&pool, &source_node_id).await?;
        let target = load_edge_endpoint(&pool, &target_node_id).await?;
        let evidence = load_edge_evidence(&pool, &edge_id).await?;

        edge.into_model(source, target, evidence)
    }

    pub async fn projection(&self, request: ProjectionRequest) -> QueryResult<GraphProjection> {
        let limit = resolve_limit(
            request.limit,
            DEFAULT_PROJECTION_LIMIT,
            self.query_service_config.max_projection_limit(),
            "limit",
        )?;
        let pool = open_read_only_pool(&self.database_path).await?;
        let nodes = load_projection_nodes(&pool, limit).await?;
        let edges = load_projection_edges(&pool, limit).await?;

        let metadata = ProjectionMetadata {
            database_path: self.database_path.display().to_string(),
            requested_limit: request.limit,
            applied_limit: limit,
            node_count: nodes.len(),
            edge_count: edges.len(),
        };

        Ok(GraphProjection {
            nodes,
            edges,
            metadata,
        })
    }

    pub async fn neighbors(&self, request: NeighborsRequest) -> QueryResult<NodeNeighbors> {
        let node_id = required_text(request.node_id, "nodeId")?;
        let relation = optional_text(request.relation, "relation")?;
        let direction = request.direction.unwrap_or(NeighborDirection::Both);
        let limit = resolve_limit(
            request.limit,
            DEFAULT_NEIGHBORS_LIMIT,
            self.query_service_config.max_neighbors_limit(),
            "limit",
        )?;
        let pool = open_read_only_pool(&self.database_path).await?;
        let node = load_node_summary(&pool, &node_id, false)
            .await?
            .ok_or_else(|| QueryError::not_found(format!("node '{node_id}' not found")))?;

        let incoming = if direction.includes_incoming() {
            load_incoming_neighbors(&pool, &node_id, relation.as_deref(), limit).await?
        } else {
            Vec::new()
        };
        let outgoing = if direction.includes_outgoing() {
            load_outgoing_neighbors(&pool, &node_id, relation.as_deref(), limit).await?
        } else {
            Vec::new()
        };

        Ok(NodeNeighbors {
            node,
            incoming,
            outgoing,
            requested_limit: request.limit,
            applied_limit: limit,
        })
    }

    pub async fn shortest_path(&self, request: ShortestPathRequest) -> QueryResult<GraphPath> {
        let source_node_id = required_text(request.source_node_id, "sourceNodeId")?;
        let target_node_id = required_text(request.target_node_id, "targetNodeId")?;
        let max_depth = resolve_limit(
            request.max_depth,
            DEFAULT_SHORTEST_PATH_DEPTH,
            self.query_service_config.max_shortest_path_depth(),
            "maxDepth",
        )?;
        let max_visited_nodes = resolve_limit(
            request.max_visited_nodes,
            DEFAULT_SHORTEST_PATH_VISITED,
            self.query_service_config.max_shortest_path_visited(),
            "maxVisitedNodes",
        )?;
        let pool = open_read_only_pool(&self.database_path).await?;

        ensure_node_exists(&pool, &source_node_id).await?;
        ensure_node_exists(&pool, &target_node_id).await?;

        if source_node_id == target_node_id {
            let Some(node) = load_node_summary(&pool, &source_node_id, false).await? else {
                return Ok(empty_path(
                    source_node_id,
                    target_node_id,
                    max_depth,
                    max_visited_nodes,
                ));
            };

            return Ok(GraphPath {
                source_node_id,
                target_node_id,
                found: true,
                nodes: vec![node],
                steps: Vec::new(),
                max_depth,
                max_visited_nodes,
            });
        }

        let Some(source_node) = load_node_summary(&pool, &source_node_id, true).await? else {
            return Ok(empty_path(
                source_node_id,
                target_node_id,
                max_depth,
                max_visited_nodes,
            ));
        };
        if load_node_summary(&pool, &target_node_id, true)
            .await?
            .is_none()
        {
            return Ok(empty_path(
                source_node_id,
                target_node_id,
                max_depth,
                max_visited_nodes,
            ));
        }

        breadth_first_path(
            &pool,
            source_node_id,
            target_node_id,
            source_node,
            max_depth,
            max_visited_nodes,
        )
        .await
    }

    pub async fn file_summary(&self, request: FileSummaryRequest) -> QueryResult<FileSummary> {
        let file_path = required_text(request.file_path, "filePath")?;
        let root_uri = optional_text(request.root_uri, "rootUri")?;
        let workspace_id = optional_positive_id(request.workspace_id, "workspaceId")?;
        let edge_limit = resolve_limit(
            request.edge_limit,
            DEFAULT_FILE_EDGE_LIMIT,
            self.query_service_config.max_file_edge_limit(),
            "edgeLimit",
        )?;
        let pool = open_read_only_pool(&self.database_path).await?;
        let (workspace_id, root_uri, file) =
            load_file_summary_file(&pool, workspace_id, root_uri.as_deref(), &file_path).await?;
        let file_node = load_file_node_summary(&pool, workspace_id, file.file_id).await?;
        let symbols = load_file_symbols(&pool, workspace_id, file.file_id).await?;
        let touching_edges =
            load_file_touching_edges(&pool, workspace_id, file.file_id, edge_limit).await?;
        let route_status_limit = self.query_service_config.max_route_status_limit();
        let file_route_statuses = load_file_route_statuses(
            &pool,
            workspace_id,
            file.file_id,
            &file.uri,
            &file.path,
            route_status_limit,
        )
        .await?;
        let workspace_route_statuses =
            load_workspace_route_statuses(&pool, workspace_id, route_status_limit).await?;

        Ok(FileSummary {
            workspace_id,
            root_uri,
            file,
            file_node,
            symbols,
            touching_edges,
            file_route_statuses,
            workspace_route_statuses,
            requested_edge_limit: request.edge_limit,
            applied_edge_limit: edge_limit,
        })
    }

    pub async fn route_status(
        &self,
        request: RouteStatusRequest,
    ) -> QueryResult<RouteStatusResults> {
        let workspace_id = optional_positive_id(request.workspace_id, "workspaceId")?;
        let root_uri = optional_text(request.root_uri, "rootUri")?;
        let route = optional_text(request.route, "route")?;
        let scope = optional_text(request.scope, "scope")?;
        let scope_key = optional_text(request.scope_key, "scopeKey")?;
        let file_path = optional_text(request.file_path, "filePath")?;
        let limit = resolve_limit(
            request.limit,
            DEFAULT_ROUTE_STATUS_LIMIT,
            self.query_service_config.max_route_status_limit(),
            "limit",
        )?;
        let pool = open_read_only_pool(&self.database_path).await?;
        let filters = RouteStatusFilters {
            workspace_id,
            root_uri: root_uri.as_deref(),
            route: route.as_deref(),
            scope: scope.as_deref(),
            scope_key: scope_key.as_deref(),
            file_path: file_path.as_deref(),
            limit,
        };
        let statuses = load_route_statuses(&pool, filters).await?;

        Ok(RouteStatusResults {
            statuses,
            requested_limit: request.limit,
            applied_limit: limit,
        })
    }
}

async fn count(pool: &SqlitePool, sql: &'static str) -> QueryResult<i64> {
    sqlx::query_scalar::<_, i64>(sql)
        .fetch_one(pool)
        .await
        .map_err(QueryError::database)
}

async fn load_latest_runs(
    pool: &SqlitePool,
    limit: i64,
) -> QueryResult<Vec<crate::ExtractionRunSummary>> {
    let rows = sqlx::query_as::<_, ExtractionRunSummaryRow>(
        r#"
        SELECT
          r.id AS run_id,
          r.workspace_id AS workspace_id,
          w.root_uri AS root_uri,
          r.provider AS provider,
          r.provider_version AS provider_version,
          r.git_commit AS git_commit,
          r.started_at AS started_at,
          r.finished_at AS finished_at,
          r.status AS status,
          r.properties_json AS properties_json
        FROM extraction_runs r
        JOIN workspaces w ON w.id = r.workspace_id
        ORDER BY
          r.started_at DESC,
          r.id DESC
        LIMIT ?
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(QueryError::database)?;

    rows.into_iter()
        .map(ExtractionRunSummaryRow::into_model)
        .collect()
}

async fn load_node_search_results(
    pool: &SqlitePool,
    query: &str,
    limit: i64,
) -> QueryResult<Vec<crate::NodeSearchResult>> {
    let contains_pattern = format!("%{}%", escape_like_pattern(query));
    let prefix_pattern = format!("{}%", escape_like_pattern(query));

    let rows = sqlx::query_as::<_, NodeSearchResultRow>(
        r#"
        SELECT
          n.id AS node_id,
          n.kind AS kind,
          COALESCE(n.display_name, n.name) AS display_label,
          n.qualified_name AS qualified_name,
          n.language AS language,
          f.path AS source_file_path,
          n.valid_to_run_id AS valid_to_run_id
        FROM nodes n
        LEFT JOIN files f ON f.id = n.file_id
        WHERE n.valid_to_run_id IS NULL
          AND (
            n.name LIKE ? ESCAPE '\'
            OR COALESCE(n.display_name, '') LIKE ? ESCAPE '\'
            OR COALESCE(n.qualified_name, '') LIKE ? ESCAPE '\'
            OR COALESCE(f.path, '') LIKE ? ESCAPE '\'
          )
        ORDER BY
          CASE
            WHEN n.name = ? COLLATE NOCASE
              OR COALESCE(n.display_name, '') = ? COLLATE NOCASE
              THEN 0
            ELSE 1
          END,
          CASE
            WHEN n.name LIKE ? ESCAPE '\'
              OR COALESCE(n.display_name, '') LIKE ? ESCAPE '\'
              THEN 0
            ELSE 1
          END,
          COALESCE(f.path, ''),
          n.kind,
          COALESCE(n.qualified_name, n.name),
          n.id
        LIMIT ?
        "#,
    )
    .bind(&contains_pattern)
    .bind(&contains_pattern)
    .bind(&contains_pattern)
    .bind(&contains_pattern)
    .bind(query)
    .bind(query)
    .bind(&prefix_pattern)
    .bind(&prefix_pattern)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(QueryError::database)?;

    Ok(rows
        .into_iter()
        .map(NodeSearchResultRow::into_model)
        .collect())
}

async fn load_node_details_row(pool: &SqlitePool, node_id: &str) -> QueryResult<NodeDetailsRow> {
    let row = sqlx::query_as::<_, NodeDetailsRow>(
        r#"
        SELECT
          n.id AS node_id,
          n.kind AS kind,
          n.name AS name,
          COALESCE(n.display_name, n.name) AS display_label,
          n.qualified_name AS qualified_name,
          n.language AS language,
          f.path AS source_file_path,
          n.start_line AS start_line,
          n.start_col AS start_col,
          n.end_line AS end_line,
          n.end_col AS end_col,
          n.selection_start_line AS selection_start_line,
          n.selection_start_col AS selection_start_col,
          container.id AS container_node_id,
          container.kind AS container_kind,
          container.name AS container_name,
          COALESCE(container.display_name, container.name) AS container_display_label,
          container.qualified_name AS container_qualified_name,
          container.language AS container_language,
          container_file.path AS container_source_file_path,
          container.valid_to_run_id AS container_valid_to_run_id,
          n.first_seen_run_id AS first_seen_run_id,
          n.last_seen_run_id AS last_seen_run_id,
          n.valid_to_run_id AS valid_to_run_id,
          n.properties_json AS properties_json,
          (
            SELECT COUNT(*)
            FROM edges incoming
            WHERE incoming.dst_node_id = n.id
              AND incoming.valid_to_run_id IS NULL
          ) AS incoming_edge_count,
          (
            SELECT COUNT(*)
            FROM edges outgoing
            WHERE outgoing.src_node_id = n.id
              AND outgoing.valid_to_run_id IS NULL
          ) AS outgoing_edge_count
        FROM nodes n
        LEFT JOIN files f ON f.id = n.file_id
        LEFT JOIN nodes container ON container.id = n.container_node_id
        LEFT JOIN files container_file ON container_file.id = container.file_id
        WHERE n.id = ?
        "#,
    )
    .bind(node_id)
    .fetch_optional(pool)
    .await
    .map_err(QueryError::database)?;

    row.ok_or_else(|| QueryError::not_found(format!("node '{node_id}' not found")))
}

async fn load_node_relation_summaries(
    pool: &SqlitePool,
    node_id: &str,
) -> QueryResult<Vec<crate::NodeRelationSummary>> {
    let rows = sqlx::query_as::<_, NodeRelationSummaryRow>(
        r#"
        SELECT
          'incoming' AS direction,
          e.relation AS relation,
          COUNT(*) AS edge_count
        FROM edges e
        WHERE e.dst_node_id = ?
          AND e.valid_to_run_id IS NULL
        GROUP BY e.relation
        UNION ALL
        SELECT
          'outgoing' AS direction,
          e.relation AS relation,
          COUNT(*) AS edge_count
        FROM edges e
        WHERE e.src_node_id = ?
          AND e.valid_to_run_id IS NULL
        GROUP BY e.relation
        ORDER BY
          direction,
          relation
        "#,
    )
    .bind(node_id)
    .bind(node_id)
    .fetch_all(pool)
    .await
    .map_err(QueryError::database)?;

    Ok(rows
        .into_iter()
        .map(NodeRelationSummaryRow::into_model)
        .collect())
}

async fn load_node_occurrences(
    pool: &SqlitePool,
    node_id: &str,
) -> QueryResult<Vec<crate::NodeOccurrence>> {
    let rows = sqlx::query_as::<_, NodeOccurrenceRow>(
        r#"
        SELECT
          o.id AS occurrence_id,
          o.run_id AS run_id,
          o.role AS role,
          f.path AS source_file_path,
          o.start_line AS start_line,
          o.start_col AS start_col,
          o.end_line AS end_line,
          o.end_col AS end_col,
          o.enclosing_node_id AS enclosing_node_id,
          o.raw_json AS raw_json
        FROM occurrences o
        JOIN files f ON f.id = o.file_id
        WHERE o.node_id = ?
        ORDER BY
          o.role,
          f.path,
          o.start_line,
          o.start_col,
          o.id
        "#,
    )
    .bind(node_id)
    .fetch_all(pool)
    .await
    .map_err(QueryError::database)?;

    rows.into_iter()
        .map(NodeOccurrenceRow::into_model)
        .collect()
}

async fn load_edge_details_row(pool: &SqlitePool, edge_id: &str) -> QueryResult<EdgeDetailsRow> {
    let row = sqlx::query_as::<_, EdgeDetailsRow>(
        r#"
        SELECT
          e.id AS edge_id,
          e.src_node_id AS source_node_id,
          e.dst_node_id AS target_node_id,
          e.relation AS relation,
          e.context AS context,
          e.confidence AS confidence,
          e.confidence_score AS confidence_score,
          e.weight AS weight,
          e.first_seen_run_id AS first_seen_run_id,
          e.last_seen_run_id AS last_seen_run_id,
          e.valid_to_run_id AS valid_to_run_id,
          e.properties_json AS properties_json
        FROM edges e
        WHERE e.id = ?
        "#,
    )
    .bind(edge_id)
    .fetch_optional(pool)
    .await
    .map_err(QueryError::database)?;

    row.ok_or_else(|| QueryError::not_found(format!("edge '{edge_id}' not found")))
}

async fn load_edge_endpoint(pool: &SqlitePool, node_id: &str) -> QueryResult<crate::EdgeEndpoint> {
    let row = sqlx::query_as::<_, EdgeEndpointRow>(
        r#"
        SELECT
          n.id AS node_id,
          n.kind AS kind,
          n.name AS name,
          COALESCE(n.display_name, n.name) AS display_label,
          n.qualified_name AS qualified_name,
          n.language AS language,
          f.path AS source_file_path,
          n.valid_to_run_id AS valid_to_run_id
        FROM nodes n
        LEFT JOIN files f ON f.id = n.file_id
        WHERE n.id = ?
        "#,
    )
    .bind(node_id)
    .fetch_optional(pool)
    .await
    .map_err(QueryError::database)?;

    let row = row.ok_or_else(|| QueryError::not_found(format!("node '{node_id}' not found")))?;
    Ok(row.into_model())
}

async fn load_edge_evidence(
    pool: &SqlitePool,
    edge_id: &str,
) -> QueryResult<Vec<crate::EdgeEvidence>> {
    let rows = sqlx::query_as::<_, EdgeEvidenceRow>(
        r#"
        SELECT
          evidence.id AS evidence_id,
          evidence.run_id AS run_id,
          evidence.provider AS provider,
          evidence.lsp_method AS lsp_method,
          f.path AS source_file_path,
          evidence.start_line AS start_line,
          evidence.start_col AS start_col,
          evidence.end_line AS end_line,
          evidence.end_col AS end_col,
          evidence.raw_json AS raw_json
        FROM edge_evidence evidence
        LEFT JOIN files f ON f.id = evidence.file_id
        WHERE evidence.edge_id = ?
        ORDER BY
          evidence.run_id,
          COALESCE(f.path, ''),
          evidence.start_line,
          evidence.start_col,
          evidence.id
        "#,
    )
    .bind(edge_id)
    .fetch_all(pool)
    .await
    .map_err(QueryError::database)?;

    rows.into_iter().map(EdgeEvidenceRow::into_model).collect()
}

async fn load_projection_nodes(pool: &SqlitePool, limit: i64) -> QueryResult<Vec<NodeSummary>> {
    let rows = sqlx::query_as::<_, NodeSummaryRow>(
        r#"
        WITH selected_symbols AS (
          SELECT n.id
          FROM nodes n
          LEFT JOIN files f ON f.id = n.file_id
          WHERE n.kind <> 'file'
            AND n.valid_to_run_id IS NULL
          ORDER BY
            COALESCE(f.path, ''),
            n.kind,
            COALESCE(n.qualified_name, n.name),
            n.id
          LIMIT ?
        ),
        selected_files AS (
          SELECT DISTINCT file_node.id
          FROM selected_symbols selected
          JOIN nodes symbol_node ON symbol_node.id = selected.id
          JOIN nodes file_node
            ON file_node.workspace_id = symbol_node.workspace_id
           AND file_node.file_id = symbol_node.file_id
           AND file_node.kind = 'file'
           AND file_node.valid_to_run_id IS NULL
          WHERE symbol_node.file_id IS NOT NULL
        ),
        selected_nodes AS (
          SELECT id FROM selected_symbols
          UNION
          SELECT id FROM selected_files
        )
        SELECT
          n.id AS node_id,
          n.kind AS kind,
          n.name AS name,
          COALESCE(n.display_name, n.name) AS display_label,
          n.qualified_name AS qualified_name,
          n.language AS language,
          f.path AS source_file_path,
          n.valid_to_run_id AS valid_to_run_id
        FROM nodes n
        JOIN selected_nodes selected ON selected.id = n.id
        LEFT JOIN files f ON f.id = n.file_id
        ORDER BY
          COALESCE(f.path, ''),
          n.kind,
          COALESCE(n.qualified_name, n.name),
          n.id
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(QueryError::database)?;

    Ok(rows.into_iter().map(NodeSummaryRow::into_model).collect())
}

async fn load_projection_edges(pool: &SqlitePool, limit: i64) -> QueryResult<Vec<EdgeSummary>> {
    let rows = sqlx::query_as::<_, EdgeSummaryRow>(
        r#"
        WITH selected_symbols AS (
          SELECT n.id
          FROM nodes n
          LEFT JOIN files f ON f.id = n.file_id
          WHERE n.kind <> 'file'
            AND n.valid_to_run_id IS NULL
          ORDER BY
            COALESCE(f.path, ''),
            n.kind,
            COALESCE(n.qualified_name, n.name),
            n.id
          LIMIT ?
        ),
        selected_files AS (
          SELECT DISTINCT file_node.id
          FROM selected_symbols selected
          JOIN nodes symbol_node ON symbol_node.id = selected.id
          JOIN nodes file_node
            ON file_node.workspace_id = symbol_node.workspace_id
           AND file_node.file_id = symbol_node.file_id
           AND file_node.kind = 'file'
           AND file_node.valid_to_run_id IS NULL
          WHERE symbol_node.file_id IS NOT NULL
        ),
        selected_nodes AS (
          SELECT id FROM selected_symbols
          UNION
          SELECT id FROM selected_files
        )
        SELECT
          e.id AS edge_id,
          e.src_node_id AS source_node_id,
          e.dst_node_id AS target_node_id,
          e.relation AS relation,
          e.context AS context,
          e.confidence AS confidence,
          e.confidence_score AS confidence_score,
          e.weight AS weight,
          e.valid_to_run_id AS valid_to_run_id
        FROM edges e
        JOIN selected_nodes src ON src.id = e.src_node_id
        JOIN selected_nodes dst ON dst.id = e.dst_node_id
        WHERE e.valid_to_run_id IS NULL
        ORDER BY
          e.relation,
          e.src_node_id,
          e.dst_node_id,
          e.id
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(QueryError::database)?;

    Ok(rows.into_iter().map(EdgeSummaryRow::into_model).collect())
}

async fn load_node_summary(
    pool: &SqlitePool,
    node_id: &str,
    active_only: bool,
) -> QueryResult<Option<NodeSummary>> {
    let sql = if active_only {
        r#"
        SELECT
          n.id AS node_id,
          n.kind AS kind,
          n.name AS name,
          COALESCE(n.display_name, n.name) AS display_label,
          n.qualified_name AS qualified_name,
          n.language AS language,
          f.path AS source_file_path,
          n.valid_to_run_id AS valid_to_run_id
        FROM nodes n
        LEFT JOIN files f ON f.id = n.file_id
        WHERE n.id = ?
          AND n.valid_to_run_id IS NULL
        "#
    } else {
        r#"
        SELECT
          n.id AS node_id,
          n.kind AS kind,
          n.name AS name,
          COALESCE(n.display_name, n.name) AS display_label,
          n.qualified_name AS qualified_name,
          n.language AS language,
          f.path AS source_file_path,
          n.valid_to_run_id AS valid_to_run_id
        FROM nodes n
        LEFT JOIN files f ON f.id = n.file_id
        WHERE n.id = ?
        "#
    };

    let row = sqlx::query_as::<_, NodeSummaryRow>(sql)
        .bind(node_id)
        .fetch_optional(pool)
        .await
        .map_err(QueryError::database)?;

    Ok(row.map(NodeSummaryRow::into_model))
}

async fn load_incoming_neighbors(
    pool: &SqlitePool,
    node_id: &str,
    relation: Option<&str>,
    limit: i64,
) -> QueryResult<Vec<NodeNeighbor>> {
    let rows = sqlx::query_as::<_, NodeNeighborRow>(
        r#"
        SELECT
          'incoming' AS direction,
          e.id AS edge_id,
          e.src_node_id AS source_node_id,
          e.dst_node_id AS target_node_id,
          e.relation AS relation,
          e.context AS context,
          e.confidence AS confidence,
          e.confidence_score AS confidence_score,
          e.weight AS weight,
          e.valid_to_run_id AS edge_valid_to_run_id,
          adjacent.id AS adjacent_node_id,
          adjacent.kind AS adjacent_kind,
          adjacent.name AS adjacent_name,
          COALESCE(adjacent.display_name, adjacent.name) AS adjacent_display_label,
          adjacent.qualified_name AS adjacent_qualified_name,
          adjacent.language AS adjacent_language,
          f.path AS adjacent_source_file_path,
          adjacent.valid_to_run_id AS adjacent_valid_to_run_id
        FROM edges e
        JOIN nodes adjacent ON adjacent.id = e.src_node_id
        LEFT JOIN files f ON f.id = adjacent.file_id
        WHERE e.dst_node_id = ?
          AND e.valid_to_run_id IS NULL
          AND adjacent.valid_to_run_id IS NULL
          AND (? IS NULL OR e.relation = ?)
        ORDER BY
          e.relation,
          COALESCE(f.path, ''),
          adjacent.kind,
          COALESCE(adjacent.qualified_name, adjacent.name),
          adjacent.id,
          e.id
        LIMIT ?
        "#,
    )
    .bind(node_id)
    .bind(relation)
    .bind(relation)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(QueryError::database)?;

    Ok(rows.into_iter().map(NodeNeighborRow::into_model).collect())
}

async fn load_outgoing_neighbors(
    pool: &SqlitePool,
    node_id: &str,
    relation: Option<&str>,
    limit: i64,
) -> QueryResult<Vec<NodeNeighbor>> {
    let rows = load_outgoing_neighbor_rows(pool, node_id, relation, limit).await?;
    Ok(rows.into_iter().map(NodeNeighborRow::into_model).collect())
}

async fn load_outgoing_neighbor_rows(
    pool: &SqlitePool,
    node_id: &str,
    relation: Option<&str>,
    limit: i64,
) -> QueryResult<Vec<NodeNeighborRow>> {
    sqlx::query_as::<_, NodeNeighborRow>(
        r#"
        SELECT
          'outgoing' AS direction,
          e.id AS edge_id,
          e.src_node_id AS source_node_id,
          e.dst_node_id AS target_node_id,
          e.relation AS relation,
          e.context AS context,
          e.confidence AS confidence,
          e.confidence_score AS confidence_score,
          e.weight AS weight,
          e.valid_to_run_id AS edge_valid_to_run_id,
          adjacent.id AS adjacent_node_id,
          adjacent.kind AS adjacent_kind,
          adjacent.name AS adjacent_name,
          COALESCE(adjacent.display_name, adjacent.name) AS adjacent_display_label,
          adjacent.qualified_name AS adjacent_qualified_name,
          adjacent.language AS adjacent_language,
          f.path AS adjacent_source_file_path,
          adjacent.valid_to_run_id AS adjacent_valid_to_run_id
        FROM edges e
        JOIN nodes adjacent ON adjacent.id = e.dst_node_id
        LEFT JOIN files f ON f.id = adjacent.file_id
        WHERE e.src_node_id = ?
          AND e.valid_to_run_id IS NULL
          AND adjacent.valid_to_run_id IS NULL
          AND (? IS NULL OR e.relation = ?)
        ORDER BY
          e.relation,
          COALESCE(f.path, ''),
          adjacent.kind,
          COALESCE(adjacent.qualified_name, adjacent.name),
          adjacent.id,
          e.id
        LIMIT ?
        "#,
    )
    .bind(node_id)
    .bind(relation)
    .bind(relation)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(QueryError::database)
}

async fn ensure_node_exists(pool: &SqlitePool, node_id: &str) -> QueryResult<()> {
    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM nodes WHERE id = ?")
        .bind(node_id)
        .fetch_one(pool)
        .await
        .map_err(QueryError::database)?;

    if exists == 0 {
        return Err(QueryError::not_found(format!("node '{node_id}' not found")));
    }

    Ok(())
}

async fn breadth_first_path(
    pool: &SqlitePool,
    source_node_id: String,
    target_node_id: String,
    source_node: NodeSummary,
    max_depth: i64,
    max_visited_nodes: i64,
) -> QueryResult<GraphPath> {
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    let mut parents: HashMap<String, (String, EdgeSummary, NodeSummary)> = HashMap::new();
    let max_visited_count = max_visited_nodes as usize;

    visited.insert(source_node_id.clone());
    queue.push_back((source_node_id.clone(), 0_i64));

    while let Some((current_node_id, depth)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }

        let rows =
            load_outgoing_neighbor_rows(pool, &current_node_id, None, max_visited_nodes).await?;
        for row in rows {
            let adjacent_node = row.adjacent_node_summary();
            if visited.contains(&adjacent_node.node_id) {
                continue;
            }

            let edge = row.edge_summary();
            parents.insert(
                adjacent_node.node_id.clone(),
                (current_node_id.clone(), edge, adjacent_node.clone()),
            );
            if adjacent_node.node_id == target_node_id {
                return Ok(build_graph_path(
                    source_node_id,
                    target_node_id,
                    source_node,
                    parents,
                    max_depth,
                    max_visited_nodes,
                ));
            }

            visited.insert(adjacent_node.node_id.clone());
            if visited.len() >= max_visited_count {
                break;
            }
            queue.push_back((adjacent_node.node_id, depth + 1));
        }

        if visited.len() >= max_visited_count {
            break;
        }
    }

    Ok(empty_path(
        source_node_id,
        target_node_id,
        max_depth,
        max_visited_nodes,
    ))
}

fn build_graph_path(
    source_node_id: String,
    target_node_id: String,
    source_node: NodeSummary,
    parents: HashMap<String, (String, EdgeSummary, NodeSummary)>,
    max_depth: i64,
    max_visited_nodes: i64,
) -> GraphPath {
    let mut current_node_id = target_node_id.clone();
    let mut reversed_steps = Vec::new();

    while current_node_id != source_node_id {
        let Some((previous_node_id, edge, node)) = parents.get(&current_node_id).cloned() else {
            return empty_path(source_node_id, target_node_id, max_depth, max_visited_nodes);
        };

        reversed_steps.push(GraphPathStep { edge, node });
        current_node_id = previous_node_id;
    }

    reversed_steps.reverse();
    let mut nodes = Vec::with_capacity(reversed_steps.len() + 1);
    nodes.push(source_node);
    nodes.extend(reversed_steps.iter().map(|step| step.node.clone()));

    GraphPath {
        source_node_id,
        target_node_id,
        found: true,
        nodes,
        steps: reversed_steps,
        max_depth,
        max_visited_nodes,
    }
}

fn empty_path(
    source_node_id: String,
    target_node_id: String,
    max_depth: i64,
    max_visited_nodes: i64,
) -> GraphPath {
    GraphPath {
        source_node_id,
        target_node_id,
        found: false,
        nodes: Vec::new(),
        steps: Vec::new(),
        max_depth,
        max_visited_nodes,
    }
}

async fn load_file_summary_file(
    pool: &SqlitePool,
    workspace_id: Option<i64>,
    root_uri: Option<&str>,
    file_path: &str,
) -> QueryResult<(i64, String, crate::FileSummaryFile)> {
    let row = sqlx::query_as::<_, FileSummaryFileRow>(
        r#"
        SELECT
          w.id AS workspace_id,
          w.root_uri AS root_uri,
          f.id AS file_id,
          f.uri AS uri,
          f.path AS path,
          f.language AS language,
          f.content_hash AS content_hash,
          f.last_seen_run_id AS last_seen_run_id,
          f.properties_json AS properties_json
        FROM files f
        JOIN workspaces w ON w.id = f.workspace_id
        WHERE f.path = ?
          AND (? IS NULL OR w.id = ?)
          AND (? IS NULL OR w.root_uri = ?)
        ORDER BY
          w.id,
          f.id
        LIMIT 1
        "#,
    )
    .bind(file_path)
    .bind(workspace_id)
    .bind(workspace_id)
    .bind(root_uri)
    .bind(root_uri)
    .fetch_optional(pool)
    .await
    .map_err(QueryError::database)?;

    let row = row.ok_or_else(|| QueryError::not_found(format!("file '{file_path}' not found")))?;
    row.into_model()
}

async fn load_file_node_summary(
    pool: &SqlitePool,
    workspace_id: i64,
    file_id: i64,
) -> QueryResult<Option<NodeSummary>> {
    let row = sqlx::query_as::<_, NodeSummaryRow>(
        r#"
        SELECT
          n.id AS node_id,
          n.kind AS kind,
          n.name AS name,
          COALESCE(n.display_name, n.name) AS display_label,
          n.qualified_name AS qualified_name,
          n.language AS language,
          f.path AS source_file_path,
          n.valid_to_run_id AS valid_to_run_id
        FROM nodes n
        LEFT JOIN files f ON f.id = n.file_id
        WHERE n.workspace_id = ?
          AND n.file_id = ?
          AND n.kind = 'file'
        ORDER BY n.id
        LIMIT 1
        "#,
    )
    .bind(workspace_id)
    .bind(file_id)
    .fetch_optional(pool)
    .await
    .map_err(QueryError::database)?;

    Ok(row.map(NodeSummaryRow::into_model))
}

async fn load_file_symbols(
    pool: &SqlitePool,
    workspace_id: i64,
    file_id: i64,
) -> QueryResult<Vec<NodeSummary>> {
    let rows = sqlx::query_as::<_, NodeSummaryRow>(
        r#"
        SELECT
          n.id AS node_id,
          n.kind AS kind,
          n.name AS name,
          COALESCE(n.display_name, n.name) AS display_label,
          n.qualified_name AS qualified_name,
          n.language AS language,
          f.path AS source_file_path,
          n.valid_to_run_id AS valid_to_run_id
        FROM nodes n
        LEFT JOIN files f ON f.id = n.file_id
        WHERE n.workspace_id = ?
          AND n.file_id = ?
          AND n.kind <> 'file'
          AND n.valid_to_run_id IS NULL
        ORDER BY
          n.kind,
          COALESCE(n.qualified_name, n.name),
          n.id
        "#,
    )
    .bind(workspace_id)
    .bind(file_id)
    .fetch_all(pool)
    .await
    .map_err(QueryError::database)?;

    Ok(rows.into_iter().map(NodeSummaryRow::into_model).collect())
}

async fn load_file_touching_edges(
    pool: &SqlitePool,
    workspace_id: i64,
    file_id: i64,
    limit: i64,
) -> QueryResult<Vec<EdgeSummary>> {
    let rows = sqlx::query_as::<_, EdgeSummaryRow>(
        r#"
        SELECT
          e.id AS edge_id,
          e.src_node_id AS source_node_id,
          e.dst_node_id AS target_node_id,
          e.relation AS relation,
          e.context AS context,
          e.confidence AS confidence,
          e.confidence_score AS confidence_score,
          e.weight AS weight,
          e.valid_to_run_id AS valid_to_run_id
        FROM edges e
        JOIN nodes src ON src.id = e.src_node_id
        JOIN nodes dst ON dst.id = e.dst_node_id
        WHERE e.workspace_id = ?
          AND e.valid_to_run_id IS NULL
          AND src.valid_to_run_id IS NULL
          AND dst.valid_to_run_id IS NULL
          AND (
            (src.file_id = ? AND src.kind <> 'file')
            OR (dst.file_id = ? AND dst.kind <> 'file')
          )
        ORDER BY
          e.relation,
          e.src_node_id,
          e.dst_node_id,
          e.id
        LIMIT ?
        "#,
    )
    .bind(workspace_id)
    .bind(file_id)
    .bind(file_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(QueryError::database)?;

    Ok(rows.into_iter().map(EdgeSummaryRow::into_model).collect())
}

async fn load_file_route_statuses(
    pool: &SqlitePool,
    workspace_id: i64,
    file_id: i64,
    file_uri: &str,
    file_path: &str,
    limit: i64,
) -> QueryResult<Vec<RouteStatus>> {
    let rows = sqlx::query_as::<_, RouteStatusRow>(
        r#"
        SELECT
          status.id AS route_status_id,
          status.workspace_id AS workspace_id,
          w.root_uri AS root_uri,
          status.route AS route,
          status.scope AS scope,
          status.scope_key AS scope_key,
          f.path AS file_path,
          status.provider AS provider,
          status.provider_version AS provider_version,
          status.content_hash AS content_hash,
          status.last_started_run_id AS last_started_run_id,
          status.last_complete_run_id AS last_complete_run_id,
          status.last_status AS last_status,
          status.diagnostics_json AS diagnostics_json,
          status.updated_at AS updated_at
        FROM extraction_route_status status
        JOIN workspaces w ON w.id = status.workspace_id
        LEFT JOIN files f ON f.id = status.file_id
        WHERE status.workspace_id = ?
          AND status.scope = 'file'
          AND (
            status.file_id = ?
            OR status.scope_key = ?
            OR status.scope_key = ?
          )
        ORDER BY
          status.route,
          status.scope,
          status.scope_key,
          status.provider,
          status.id
        LIMIT ?
        "#,
    )
    .bind(workspace_id)
    .bind(file_id)
    .bind(file_uri)
    .bind(file_path)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(QueryError::database)?;

    rows.into_iter().map(RouteStatusRow::into_model).collect()
}

async fn load_workspace_route_statuses(
    pool: &SqlitePool,
    workspace_id: i64,
    limit: i64,
) -> QueryResult<Vec<RouteStatus>> {
    let rows = sqlx::query_as::<_, RouteStatusRow>(
        r#"
        SELECT
          status.id AS route_status_id,
          status.workspace_id AS workspace_id,
          w.root_uri AS root_uri,
          status.route AS route,
          status.scope AS scope,
          status.scope_key AS scope_key,
          f.path AS file_path,
          status.provider AS provider,
          status.provider_version AS provider_version,
          status.content_hash AS content_hash,
          status.last_started_run_id AS last_started_run_id,
          status.last_complete_run_id AS last_complete_run_id,
          status.last_status AS last_status,
          status.diagnostics_json AS diagnostics_json,
          status.updated_at AS updated_at
        FROM extraction_route_status status
        JOIN workspaces w ON w.id = status.workspace_id
        LEFT JOIN files f ON f.id = status.file_id
        WHERE status.workspace_id = ?
          AND status.scope = 'workspace'
        ORDER BY
          status.route,
          status.scope,
          status.scope_key,
          status.provider,
          status.id
        LIMIT ?
        "#,
    )
    .bind(workspace_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(QueryError::database)?;

    rows.into_iter().map(RouteStatusRow::into_model).collect()
}

async fn load_route_statuses(
    pool: &SqlitePool,
    filters: RouteStatusFilters<'_>,
) -> QueryResult<Vec<RouteStatus>> {
    let rows = sqlx::query_as::<_, RouteStatusRow>(
        r#"
        SELECT
          status.id AS route_status_id,
          status.workspace_id AS workspace_id,
          w.root_uri AS root_uri,
          status.route AS route,
          status.scope AS scope,
          status.scope_key AS scope_key,
          f.path AS file_path,
          status.provider AS provider,
          status.provider_version AS provider_version,
          status.content_hash AS content_hash,
          status.last_started_run_id AS last_started_run_id,
          status.last_complete_run_id AS last_complete_run_id,
          status.last_status AS last_status,
          status.diagnostics_json AS diagnostics_json,
          status.updated_at AS updated_at
        FROM extraction_route_status status
        JOIN workspaces w ON w.id = status.workspace_id
        LEFT JOIN files f ON f.id = status.file_id
        WHERE (? IS NULL OR status.workspace_id = ?)
          AND (? IS NULL OR w.root_uri = ?)
          AND (? IS NULL OR status.route = ?)
          AND (? IS NULL OR status.scope = ?)
          AND (? IS NULL OR status.scope_key = ?)
          AND (
            ? IS NULL
            OR f.path = ?
            OR status.scope_key = ?
          )
        ORDER BY
          status.workspace_id,
          status.route,
          status.scope,
          status.scope_key,
          status.provider,
          status.id
        LIMIT ?
        "#,
    )
    .bind(filters.workspace_id)
    .bind(filters.workspace_id)
    .bind(filters.root_uri)
    .bind(filters.root_uri)
    .bind(filters.route)
    .bind(filters.route)
    .bind(filters.scope)
    .bind(filters.scope)
    .bind(filters.scope_key)
    .bind(filters.scope_key)
    .bind(filters.file_path)
    .bind(filters.file_path)
    .bind(filters.file_path)
    .bind(filters.limit)
    .fetch_all(pool)
    .await
    .map_err(QueryError::database)?;

    rows.into_iter().map(RouteStatusRow::into_model).collect()
}

fn resolve_limit(
    requested: Option<i64>,
    default_value: i64,
    maximum: i64,
    field_name: &str,
) -> QueryResult<i64> {
    let limit = requested.unwrap_or(default_value);

    if !(1..=maximum).contains(&limit) {
        return Err(QueryError::invalid_params(format!(
            "{field_name} must be between 1 and {maximum}"
        )));
    }

    Ok(limit)
}

fn required_text(value: String, field_name: &str) -> QueryResult<String> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return Err(QueryError::invalid_params(format!(
            "{field_name} must not be blank"
        )));
    }

    Ok(trimmed.to_string())
}

fn optional_text(value: Option<String>, field_name: &str) -> QueryResult<Option<String>> {
    match value {
        Some(value) => required_text(value, field_name).map(Some),
        None => Ok(None),
    }
}

fn optional_positive_id(value: Option<i64>, field_name: &str) -> QueryResult<Option<i64>> {
    match value {
        Some(value) if value <= 0 => Err(QueryError::invalid_params(format!(
            "{field_name} must be positive"
        ))),
        Some(value) => Ok(Some(value)),
        None => Ok(None),
    }
}
