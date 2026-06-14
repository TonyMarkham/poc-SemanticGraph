use crate::{
    VisualizerServerError, VisualizerServerResult,
    dto::{GraphEdgeDto, GraphMetadataDto, GraphNodeDto, GraphProjectionDto},
    projection::{graph_edge_row::GraphEdgeRow, graph_node_row::GraphNodeRow},
};

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct GraphProjectionService {
    database_path: PathBuf,
}

impl GraphProjectionService {
    pub fn new(database_path: PathBuf) -> Self {
        Self { database_path }
    }

    pub async fn projection(&self, limit: i64) -> VisualizerServerResult<GraphProjectionDto> {
        let pool = open_read_only_pool(&self.database_path).await?;
        let nodes = load_nodes(&pool, limit).await?;
        let edges = load_edges(&pool, limit).await?;

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
}

async fn open_read_only_pool(path: &Path) -> VisualizerServerResult<SqlitePool> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .read_only(true)
        .foreign_keys(true);

    SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .map_err(VisualizerServerError::database)
}

async fn load_nodes(pool: &SqlitePool, limit: i64) -> VisualizerServerResult<Vec<GraphNodeDto>> {
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

async fn load_edges(pool: &SqlitePool, limit: i64) -> VisualizerServerResult<Vec<GraphEdgeDto>> {
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
