use crate::{
    VisualizerServerError, VisualizerServerResult,
    dto::{
        GraphEdgeDetailsDto, GraphEdgeDto, GraphEdgeEndpointDto, GraphEdgeEvidenceDto,
        GraphMetadataDto, GraphNodeDetailsDto, GraphNodeDto, GraphNodeOccurrenceDto,
        GraphNodeRelationSummaryDto, GraphNodeSearchResultDto, GraphNodeSearchResultsDto,
        GraphProjectionDto,
    },
    query::{
        graph_edge_details_row::GraphEdgeDetailsRow, graph_edge_endpoint_row::GraphEdgeEndpointRow,
        graph_edge_evidence_row::GraphEdgeEvidenceRow, graph_edge_row::GraphEdgeRow,
        graph_node_details_row::GraphNodeDetailsRow,
        graph_node_occurrence_row::GraphNodeOccurrenceRow,
        graph_node_relation_summary_row::GraphNodeRelationSummaryRow,
        graph_node_row::GraphNodeRow, graph_node_search_result_row::GraphNodeSearchResultRow,
        sqlite_read_pool::open_read_only_pool,
    },
};

use serde_json::Value;
use sqlx::SqlitePool;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct GraphQueryService {
    database_path: PathBuf,
}

impl GraphQueryService {
    pub fn new(database_path: PathBuf) -> Self {
        Self { database_path }
    }

    pub async fn projection(&self, limit: i64) -> VisualizerServerResult<GraphProjectionDto> {
        let pool = open_read_only_pool(&self.database_path).await?;
        let nodes = load_projection_nodes(&pool, limit).await?;
        let edges = load_projection_edges(&pool, limit).await?;

        let metadata = GraphMetadataDto {
            database_path: self.database_path.display().to_string(),
            limit,
            node_count: nodes.len(),
            edge_count: edges.len(),
        };

        Ok(GraphProjectionDto {
            nodes,
            edges,
            metadata,
        })
    }

    pub async fn node_details(
        &self,
        node_id: &str,
    ) -> VisualizerServerResult<GraphNodeDetailsDto> {
        let pool = open_read_only_pool(&self.database_path).await?;
        let node = load_node_details_row(&pool, node_id).await?;
        let relations = load_node_relation_summaries(&pool, node_id).await?;
        let occurrences = load_node_occurrences(&pool, node_id).await?;

        Ok(GraphNodeDetailsDto {
            node_id: node.node_id,
            kind: node.kind,
            name: node.name,
            display_label: node.display_label,
            qualified_name: node.qualified_name,
            language: node.language,
            source_file_path: node.source_file_path,
            start_line: node.start_line,
            start_col: node.start_col,
            end_line: node.end_line,
            end_col: node.end_col,
            selection_start_line: node.selection_start_line,
            selection_start_col: node.selection_start_col,
            container_node_id: node.container_node_id,
            container_display_label: node.container_display_label,
            first_seen_run_id: node.first_seen_run_id,
            last_seen_run_id: node.last_seen_run_id,
            properties_json: parse_json_value(&node.properties_json)?,
            incoming_edge_count: node.incoming_edge_count,
            outgoing_edge_count: node.outgoing_edge_count,
            relations,
            occurrences,
        })
    }

    pub async fn edge_details(
        &self,
        edge_id: &str,
    ) -> VisualizerServerResult<GraphEdgeDetailsDto> {
        let pool = open_read_only_pool(&self.database_path).await?;
        let edge = load_edge_details_row(&pool, edge_id).await?;
        let source = load_edge_endpoint(&pool, &edge.source_node_id).await?;
        let target = load_edge_endpoint(&pool, &edge.target_node_id).await?;
        let evidence = load_edge_evidence(&pool, edge_id).await?;

        Ok(GraphEdgeDetailsDto {
            edge_id: edge.edge_id,
            relation: edge.relation,
            context: edge.context,
            confidence: edge.confidence,
            confidence_score: edge.confidence_score,
            weight: edge.weight,
            first_seen_run_id: edge.first_seen_run_id,
            last_seen_run_id: edge.last_seen_run_id,
            properties_json: parse_json_value(&edge.properties_json)?,
            source,
            target,
            evidence,
        })
    }

    pub async fn search_nodes(
        &self,
        query: &str,
        limit: i64,
    ) -> VisualizerServerResult<GraphNodeSearchResultsDto> {
        let pool = open_read_only_pool(&self.database_path).await?;
        let results = load_node_search_results(&pool, query, limit).await?;

        Ok(GraphNodeSearchResultsDto { results })
    }
}

async fn load_projection_nodes(
    pool: &SqlitePool,
    limit: i64,
) -> VisualizerServerResult<Vec<GraphNodeDto>> {
    let rows = sqlx::query_as::<_, GraphNodeRow>(
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
          WHERE symbol_node.file_id IS NOT NULL
        ),
        selected_nodes AS (
          SELECT id FROM selected_symbols
          UNION
          SELECT id FROM selected_files
        )
        SELECT
          n.id AS id,
          n.kind AS kind,
          COALESCE(n.display_name, n.name) AS display_label,
          n.qualified_name AS qualified_name,
          n.language AS language,
          f.path AS source_file_path
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
    .map_err(VisualizerServerError::database)?;

    Ok(rows
        .into_iter()
        .map(|row| GraphNodeDto {
            id: row.id,
            kind: row.kind,
            display_label: row.display_label,
            qualified_name: row.qualified_name,
            language: row.language,
            source_file_path: row.source_file_path,
        })
        .collect())
}

async fn load_projection_edges(
    pool: &SqlitePool,
    limit: i64,
) -> VisualizerServerResult<Vec<GraphEdgeDto>> {
    let rows = sqlx::query_as::<_, GraphEdgeRow>(
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
          WHERE symbol_node.file_id IS NOT NULL
        ),
        selected_nodes AS (
          SELECT id FROM selected_symbols
          UNION
          SELECT id FROM selected_files
        )
        SELECT
          e.id AS id,
          e.src_node_id AS source_node_id,
          e.dst_node_id AS target_node_id,
          e.relation AS relation,
          e.confidence AS confidence,
          e.confidence_score AS confidence_score
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
    .map_err(VisualizerServerError::database)?;

    Ok(rows
        .into_iter()
        .map(|row| GraphEdgeDto {
            id: row.id,
            source_node_id: row.source_node_id,
            target_node_id: row.target_node_id,
            relation: row.relation,
            confidence: row.confidence,
            confidence_score: row.confidence_score,
        })
        .collect())
}

async fn load_node_details_row(
    pool: &SqlitePool,
    node_id: &str,
) -> VisualizerServerResult<GraphNodeDetailsRow> {
    let row = sqlx::query_as::<_, GraphNodeDetailsRow>(
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
          n.container_node_id AS container_node_id,
          COALESCE(container.display_name, container.name) AS container_display_label,
          n.first_seen_run_id AS first_seen_run_id,
          n.last_seen_run_id AS last_seen_run_id,
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
        WHERE n.id = ?
        "#,
    )
    .bind(node_id)
    .fetch_optional(pool)
    .await
    .map_err(VisualizerServerError::database)?;

    row.ok_or_else(|| VisualizerServerError::not_found(format!("node '{node_id}' not found")))
}

async fn load_node_relation_summaries(
    pool: &SqlitePool,
    node_id: &str,
) -> VisualizerServerResult<Vec<GraphNodeRelationSummaryDto>> {
    let rows = sqlx::query_as::<_, GraphNodeRelationSummaryRow>(
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
    .map_err(VisualizerServerError::database)?;

    Ok(rows
        .into_iter()
        .map(|row| GraphNodeRelationSummaryDto {
            direction: row.direction,
            relation: row.relation,
            edge_count: row.edge_count,
        })
        .collect())
}

async fn load_node_occurrences(
    pool: &SqlitePool,
    node_id: &str,
) -> VisualizerServerResult<Vec<GraphNodeOccurrenceDto>> {
    let rows = sqlx::query_as::<_, GraphNodeOccurrenceRow>(
        r#"
        SELECT
          o.id AS id,
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
    .map_err(VisualizerServerError::database)?;

    rows.into_iter()
        .map(|row| {
            Ok(GraphNodeOccurrenceDto {
                id: row.id,
                run_id: row.run_id,
                role: row.role,
                source_file_path: row.source_file_path,
                start_line: row.start_line,
                start_col: row.start_col,
                end_line: row.end_line,
                end_col: row.end_col,
                enclosing_node_id: row.enclosing_node_id,
                raw_json: parse_optional_json_value(row.raw_json)?,
            })
        })
        .collect()
}

async fn load_edge_details_row(
    pool: &SqlitePool,
    edge_id: &str,
) -> VisualizerServerResult<GraphEdgeDetailsRow> {
    let row = sqlx::query_as::<_, GraphEdgeDetailsRow>(
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
          e.properties_json AS properties_json
        FROM edges e
        WHERE e.id = ?
        "#,
    )
    .bind(edge_id)
    .fetch_optional(pool)
    .await
    .map_err(VisualizerServerError::database)?;

    row.ok_or_else(|| VisualizerServerError::not_found(format!("edge '{edge_id}' not found")))
}

async fn load_edge_endpoint(
    pool: &SqlitePool,
    node_id: &str,
) -> VisualizerServerResult<GraphEdgeEndpointDto> {
    let row = sqlx::query_as::<_, GraphEdgeEndpointRow>(
        r#"
        SELECT
          n.id AS node_id,
          n.kind AS kind,
          COALESCE(n.display_name, n.name) AS display_label,
          n.qualified_name AS qualified_name,
          n.language AS language,
          f.path AS source_file_path
        FROM nodes n
        LEFT JOIN files f ON f.id = n.file_id
        WHERE n.id = ?
        "#,
    )
    .bind(node_id)
    .fetch_optional(pool)
    .await
    .map_err(VisualizerServerError::database)?;

    let row =
        row.ok_or_else(|| VisualizerServerError::not_found(format!("node '{node_id}' not found")))?;

    Ok(GraphEdgeEndpointDto {
        node_id: row.node_id,
        kind: row.kind,
        display_label: row.display_label,
        qualified_name: row.qualified_name,
        language: row.language,
        source_file_path: row.source_file_path,
    })
}

async fn load_edge_evidence(
    pool: &SqlitePool,
    edge_id: &str,
) -> VisualizerServerResult<Vec<GraphEdgeEvidenceDto>> {
    let rows = sqlx::query_as::<_, GraphEdgeEvidenceRow>(
        r#"
        SELECT
          evidence.id AS id,
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
    .map_err(VisualizerServerError::database)?;

    rows.into_iter()
        .map(|row| {
            Ok(GraphEdgeEvidenceDto {
                id: row.id,
                run_id: row.run_id,
                provider: row.provider,
                lsp_method: row.lsp_method,
                source_file_path: row.source_file_path,
                start_line: row.start_line,
                start_col: row.start_col,
                end_line: row.end_line,
                end_col: row.end_col,
                raw_json: parse_optional_json_value(row.raw_json)?,
            })
        })
        .collect()
}

async fn load_node_search_results(
    pool: &SqlitePool,
    query: &str,
    limit: i64,
) -> VisualizerServerResult<Vec<GraphNodeSearchResultDto>> {
    let contains_pattern = format!("%{}%", escape_like_pattern(query));
    let prefix_pattern = format!("{}%", escape_like_pattern(query));

    let rows = sqlx::query_as::<_, GraphNodeSearchResultRow>(
        r#"
        SELECT
          n.id AS node_id,
          n.kind AS kind,
          COALESCE(n.display_name, n.name) AS display_label,
          n.qualified_name AS qualified_name,
          n.language AS language,
          f.path AS source_file_path
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
    .map_err(VisualizerServerError::database)?;

    Ok(rows
        .into_iter()
        .map(|row| GraphNodeSearchResultDto {
            node_id: row.node_id,
            kind: row.kind,
            display_label: row.display_label,
            qualified_name: row.qualified_name,
            language: row.language,
            source_file_path: row.source_file_path,
        })
        .collect())
}

fn parse_json_value(raw_json: &str) -> VisualizerServerResult<Value> {
    serde_json::from_str(raw_json).map_err(VisualizerServerError::json)
}

fn parse_optional_json_value(raw_json: Option<String>) -> VisualizerServerResult<Option<Value>> {
    match raw_json {
        Some(value) => parse_json_value(&value).map(Some),
        None => Ok(None),
    }
}

fn escape_like_pattern(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());

    for character in value.chars() {
        match character {
            '%' | '_' | '\\' => {
                escaped.push('\\');
                escaped.push(character);
            }
            _ => escaped.push(character),
        }
    }

    escaped
}
