use crate::projection::GraphProjectionService;

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use std::{
    error::Error,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[tokio::test]
async fn projection_includes_selected_symbols_files_and_edges() -> Result<(), Box<dyn Error>> {
    let database_path = temp_database_path()?;
    let pool = create_fixture_database(&database_path).await?;
    seed_fixture_database(&pool).await?;
    pool.close().await;

    let service = GraphProjectionService::new(database_path.clone());
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

async fn create_fixture_database(database_path: &PathBuf) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(database_path)
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;

    sqlx::raw_sql(
        r#"
        CREATE TABLE files (
          id INTEGER PRIMARY KEY,
          path TEXT NOT NULL
        );

        CREATE TABLE nodes (
          id TEXT PRIMARY KEY,
          workspace_id INTEGER NOT NULL,
          language TEXT NOT NULL,
          kind TEXT NOT NULL,
          name TEXT NOT NULL,
          qualified_name TEXT,
          display_name TEXT,
          file_id INTEGER,
          valid_to_run_id INTEGER
        );

        CREATE TABLE edges (
          id TEXT PRIMARY KEY,
          src_node_id TEXT NOT NULL,
          dst_node_id TEXT NOT NULL,
          relation TEXT NOT NULL,
          confidence TEXT NOT NULL,
          confidence_score REAL NOT NULL,
          valid_to_run_id INTEGER
        );
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}

async fn seed_fixture_database(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO files (id, path) VALUES (1, 'src/lib.rs')")
        .execute(pool)
        .await?;

    sqlx::query(
        r#"
        INSERT INTO nodes (
          id,
          workspace_id,
          language,
          kind,
          name,
          qualified_name,
          display_name,
          file_id,
          valid_to_run_id
        )
        VALUES
          ('file-src-lib', 1, 'rust', 'file', 'lib.rs', 'src/lib.rs', 'lib.rs', 1, NULL),
          ('symbol-run', 1, 'rust', 'function', 'run', 'crate::run', 'run', 1, NULL),
          ('symbol-hidden', 1, 'rust', 'function', 'hidden', 'crate::z_hidden', 'hidden', 1, NULL)
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO edges (
          id,
          src_node_id,
          dst_node_id,
          relation,
          confidence,
          confidence_score,
          valid_to_run_id
        )
        VALUES
          ('edge-file-run', 'file-src-lib', 'symbol-run', 'contains', 'EXTRACTED', 1.0, NULL),
          ('edge-file-hidden', 'file-src-lib', 'symbol-hidden', 'contains', 'EXTRACTED', 1.0, NULL)
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
