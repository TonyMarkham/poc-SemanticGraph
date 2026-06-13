use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Executor, SqlitePool};

use crate::Result;
use crate::ids::{edge_id, node_id};

#[derive(Debug, Clone)]
pub struct GraphStore {
    pool: SqlitePool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextRange {
    pub start_line: i64,
    pub start_col: i64,
    pub end_line: i64,
    pub end_col: i64,
}

#[derive(Debug, Clone)]
pub struct FileInput<'a> {
    pub workspace_id: i64,
    pub uri: &'a str,
    pub path: &'a str,
    pub language: &'a str,
    pub content_hash: Option<&'a str>,
    pub last_seen_run_id: Option<i64>,
    pub properties_json: Value,
}

#[derive(Debug, Clone)]
pub struct NodeInput<'a> {
    pub workspace_id: i64,
    pub language: &'a str,
    pub kind: &'a str,
    pub name: &'a str,
    pub qualified_name: Option<&'a str>,
    pub display_name: Option<&'a str>,
    pub symbol_key: &'a str,
    pub file_id: Option<i64>,
    pub range: Option<TextRange>,
    pub selection_range: Option<TextRange>,
    pub container_node_id: Option<&'a str>,
    pub properties_json: Value,
    pub run_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct EdgeInput<'a> {
    pub workspace_id: i64,
    pub src_node_id: &'a str,
    pub dst_node_id: &'a str,
    pub relation: &'a str,
    pub context: Option<&'a str>,
    pub confidence: &'a str,
    pub confidence_score: f64,
    pub weight: f64,
    pub properties_json: Value,
    pub run_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct OccurrenceInput<'a> {
    pub node_id: &'a str,
    pub run_id: i64,
    pub file_id: i64,
    pub role: &'a str,
    pub range: TextRange,
    pub enclosing_node_id: Option<&'a str>,
    pub raw_json: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct EdgeEvidenceInput<'a> {
    pub edge_id: &'a str,
    pub run_id: i64,
    pub provider: &'a str,
    pub lsp_method: Option<&'a str>,
    pub file_id: Option<i64>,
    pub range: Option<TextRange>,
    pub raw_json: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphStoreStats {
    pub workspaces: i64,
    pub extraction_runs: i64,
    pub files: i64,
    pub nodes: i64,
    pub edges: i64,
    pub occurrences: i64,
    pub edge_evidence: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DemoSeedSummary {
    pub workspace_id: i64,
    pub run_id: i64,
    pub file_id: i64,
    pub caller_node_id: String,
    pub callee_node_id: String,
    pub edge_id: String,
}

impl GraphStore {
    pub async fn connect(path: impl AsRef<Path>) -> Result<Self> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;

        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> Result<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        self.pool.execute("PRAGMA foreign_keys = ON").await?;
        Ok(())
    }

    pub async fn create_workspace(&self, root_uri: &str, kind: &str) -> Result<i64> {
        sqlx::query(
            r#"
            INSERT INTO workspaces (root_uri, kind)
            VALUES (?, ?)
            ON CONFLICT(root_uri) DO UPDATE SET kind = excluded.kind
            "#,
        )
        .bind(root_uri)
        .bind(kind)
        .execute(&self.pool)
        .await?;

        let id = sqlx::query_scalar("SELECT id FROM workspaces WHERE root_uri = ?")
            .bind(root_uri)
            .fetch_one(&self.pool)
            .await?;

        Ok(id)
    }

    pub async fn start_run(
        &self,
        workspace_id: i64,
        provider: &str,
        provider_version: Option<&str>,
        git_commit: Option<&str>,
    ) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO extraction_runs (
              workspace_id,
              provider,
              provider_version,
              git_commit
            )
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(workspace_id)
        .bind(provider)
        .bind(provider_version)
        .bind(git_commit)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn finish_run(&self, run_id: i64, status: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE extraction_runs
            SET finished_at = CURRENT_TIMESTAMP,
                status = ?
            WHERE id = ?
            "#,
        )
        .bind(status)
        .bind(run_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn upsert_file(&self, input: FileInput<'_>) -> Result<i64> {
        let properties_json = input.properties_json.to_string();

        sqlx::query(
            r#"
            INSERT INTO files (
              workspace_id,
              uri,
              path,
              language,
              content_hash,
              last_seen_run_id,
              properties_json
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(workspace_id, uri) DO UPDATE SET
              path = excluded.path,
              language = excluded.language,
              content_hash = excluded.content_hash,
              last_seen_run_id = excluded.last_seen_run_id,
              properties_json = excluded.properties_json
            "#,
        )
        .bind(input.workspace_id)
        .bind(input.uri)
        .bind(input.path)
        .bind(input.language)
        .bind(input.content_hash)
        .bind(input.last_seen_run_id)
        .bind(properties_json)
        .execute(&self.pool)
        .await?;

        let id = sqlx::query_scalar("SELECT id FROM files WHERE workspace_id = ? AND uri = ?")
            .bind(input.workspace_id)
            .bind(input.uri)
            .fetch_one(&self.pool)
            .await?;

        Ok(id)
    }

    pub async fn upsert_node(&self, input: NodeInput<'_>) -> Result<String> {
        let id = node_id(input.workspace_id, input.language, input.symbol_key);
        let properties_json = input.properties_json.to_string();
        let range = input.range;
        let selection_range = input.selection_range;

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
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL)
            ON CONFLICT(id) DO UPDATE SET
              kind = excluded.kind,
              name = excluded.name,
              qualified_name = excluded.qualified_name,
              display_name = excluded.display_name,
              file_id = excluded.file_id,
              start_line = excluded.start_line,
              start_col = excluded.start_col,
              end_line = excluded.end_line,
              end_col = excluded.end_col,
              selection_start_line = excluded.selection_start_line,
              selection_start_col = excluded.selection_start_col,
              container_node_id = excluded.container_node_id,
              properties_json = excluded.properties_json,
              last_seen_run_id = excluded.last_seen_run_id,
              valid_to_run_id = NULL
            "#,
        )
        .bind(&id)
        .bind(input.workspace_id)
        .bind(input.language)
        .bind(input.kind)
        .bind(input.name)
        .bind(input.qualified_name)
        .bind(input.display_name)
        .bind(input.symbol_key)
        .bind(input.file_id)
        .bind(range.map(|value| value.start_line))
        .bind(range.map(|value| value.start_col))
        .bind(range.map(|value| value.end_line))
        .bind(range.map(|value| value.end_col))
        .bind(selection_range.map(|value| value.start_line))
        .bind(selection_range.map(|value| value.start_col))
        .bind(input.container_node_id)
        .bind(properties_json)
        .bind(input.run_id)
        .bind(input.run_id)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    pub async fn upsert_edge(&self, input: EdgeInput<'_>) -> Result<String> {
        let id = edge_id(
            input.workspace_id,
            input.src_node_id,
            input.dst_node_id,
            input.relation,
            input.context,
        );
        let properties_json = input.properties_json.to_string();

        sqlx::query(
            r#"
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
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL)
            ON CONFLICT(id) DO UPDATE SET
              confidence = excluded.confidence,
              confidence_score = excluded.confidence_score,
              weight = excluded.weight,
              properties_json = excluded.properties_json,
              last_seen_run_id = excluded.last_seen_run_id,
              valid_to_run_id = NULL
            "#,
        )
        .bind(&id)
        .bind(input.workspace_id)
        .bind(input.src_node_id)
        .bind(input.dst_node_id)
        .bind(input.relation)
        .bind(input.context)
        .bind(input.confidence)
        .bind(input.confidence_score)
        .bind(input.weight)
        .bind(properties_json)
        .bind(input.run_id)
        .bind(input.run_id)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    pub async fn insert_occurrence(&self, input: OccurrenceInput<'_>) -> Result<i64> {
        let raw_json = input.raw_json.map(|value| value.to_string());

        let result = sqlx::query(
            r#"
            INSERT INTO occurrences (
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
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(input.node_id)
        .bind(input.run_id)
        .bind(input.file_id)
        .bind(input.role)
        .bind(input.range.start_line)
        .bind(input.range.start_col)
        .bind(input.range.end_line)
        .bind(input.range.end_col)
        .bind(input.enclosing_node_id)
        .bind(raw_json)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn insert_edge_evidence(&self, input: EdgeEvidenceInput<'_>) -> Result<i64> {
        let raw_json = input.raw_json.map(|value| value.to_string());
        let range = input.range;

        let result = sqlx::query(
            r#"
            INSERT INTO edge_evidence (
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
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(input.edge_id)
        .bind(input.run_id)
        .bind(input.provider)
        .bind(input.lsp_method)
        .bind(input.file_id)
        .bind(range.map(|value| value.start_line))
        .bind(range.map(|value| value.start_col))
        .bind(range.map(|value| value.end_line))
        .bind(range.map(|value| value.end_col))
        .bind(raw_json)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn demo_seed(&self, root_uri: &str) -> Result<DemoSeedSummary> {
        let workspace_id = self.create_workspace(root_uri, "rust").await?;
        let run_id = self
            .start_run(workspace_id, "demo", Some(env!("CARGO_PKG_VERSION")), None)
            .await?;
        let file_uri = format!("{}/src/lib.rs", root_uri.trim_end_matches('/'));
        let file_id = self
            .upsert_file(FileInput {
                workspace_id,
                uri: &file_uri,
                path: "src/lib.rs",
                language: "rust",
                content_hash: None,
                last_seen_run_id: Some(run_id),
                properties_json: json!({}),
            })
            .await?;

        let caller_range = TextRange {
            start_line: 1,
            start_col: 0,
            end_line: 3,
            end_col: 1,
        };
        let callee_range = TextRange {
            start_line: 5,
            start_col: 0,
            end_line: 7,
            end_col: 1,
        };
        let caller_symbol_key = format!("{file_uri}#function:caller:1:0");
        let callee_symbol_key = format!("{file_uri}#function:callee:5:0");
        let caller_node_id = self
            .upsert_node(NodeInput {
                workspace_id,
                language: "rust",
                kind: "function",
                name: "caller",
                qualified_name: Some("demo::caller"),
                display_name: Some("caller"),
                symbol_key: &caller_symbol_key,
                file_id: Some(file_id),
                range: Some(caller_range),
                selection_range: Some(TextRange {
                    start_line: 1,
                    start_col: 3,
                    end_line: 1,
                    end_col: 9,
                }),
                container_node_id: None,
                properties_json: json!({}),
                run_id: Some(run_id),
            })
            .await?;
        let callee_node_id = self
            .upsert_node(NodeInput {
                workspace_id,
                language: "rust",
                kind: "function",
                name: "callee",
                qualified_name: Some("demo::callee"),
                display_name: Some("callee"),
                symbol_key: &callee_symbol_key,
                file_id: Some(file_id),
                range: Some(callee_range),
                selection_range: Some(TextRange {
                    start_line: 5,
                    start_col: 3,
                    end_line: 5,
                    end_col: 9,
                }),
                container_node_id: None,
                properties_json: json!({}),
                run_id: Some(run_id),
            })
            .await?;
        let edge_id = self
            .upsert_edge(EdgeInput {
                workspace_id,
                src_node_id: &caller_node_id,
                dst_node_id: &callee_node_id,
                relation: "calls",
                context: None,
                confidence: "EXTRACTED",
                confidence_score: 1.0,
                weight: 1.0,
                properties_json: json!({}),
                run_id: Some(run_id),
            })
            .await?;

        self.insert_occurrence(OccurrenceInput {
            node_id: &caller_node_id,
            run_id,
            file_id,
            role: "definition",
            range: caller_range,
            enclosing_node_id: None,
            raw_json: Some(json!({ "source": "demo-seed" })),
        })
        .await?;
        self.insert_edge_evidence(EdgeEvidenceInput {
            edge_id: &edge_id,
            run_id,
            provider: "demo",
            lsp_method: Some("demo/calls"),
            file_id: Some(file_id),
            range: Some(TextRange {
                start_line: 2,
                start_col: 4,
                end_line: 2,
                end_col: 12,
            }),
            raw_json: Some(json!({ "source": "demo-seed" })),
        })
        .await?;
        self.finish_run(run_id, "complete").await?;

        Ok(DemoSeedSummary {
            workspace_id,
            run_id,
            file_id,
            caller_node_id,
            callee_node_id,
            edge_id,
        })
    }

    pub async fn stats(&self) -> Result<GraphStoreStats> {
        Ok(GraphStoreStats {
            workspaces: sqlx::query_scalar("SELECT COUNT(*) FROM workspaces")
                .fetch_one(&self.pool)
                .await?,
            extraction_runs: sqlx::query_scalar("SELECT COUNT(*) FROM extraction_runs")
                .fetch_one(&self.pool)
                .await?,
            files: sqlx::query_scalar("SELECT COUNT(*) FROM files")
                .fetch_one(&self.pool)
                .await?,
            nodes: sqlx::query_scalar("SELECT COUNT(*) FROM nodes")
                .fetch_one(&self.pool)
                .await?,
            edges: sqlx::query_scalar("SELECT COUNT(*) FROM edges")
                .fetch_one(&self.pool)
                .await?,
            occurrences: sqlx::query_scalar("SELECT COUNT(*) FROM occurrences")
                .fetch_one(&self.pool)
                .await?,
            edge_evidence: sqlx::query_scalar("SELECT COUNT(*) FROM edge_evidence")
                .fetch_one(&self.pool)
                .await?,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::error::Error;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde_json::json;

    use super::*;

    fn temp_db_path() -> std::result::Result<PathBuf, Box<dyn Error>> {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        Ok(env::temp_dir().join(format!(
            "poc-semanticgraph-store-{}-{stamp}.db",
            std::process::id()
        )))
    }

    async fn migrated_store() -> std::result::Result<GraphStore, Box<dyn Error>> {
        let path = temp_db_path()?;
        let store = GraphStore::connect(path).await?;
        store.migrate().await?;
        Ok(store)
    }

    #[tokio::test]
    async fn migration_creates_empty_core_schema() -> std::result::Result<(), Box<dyn Error>> {
        let store = migrated_store().await?;

        assert_eq!(
            store.stats().await?,
            GraphStoreStats {
                workspaces: 0,
                extraction_runs: 0,
                files: 0,
                nodes: 0,
                edges: 0,
                occurrences: 0,
                edge_evidence: 0,
            }
        );

        let index_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM sqlite_master
            WHERE type = 'index'
              AND name IN (
                'idx_files_workspace_path',
                'idx_nodes_workspace_qname',
                'idx_nodes_file',
                'idx_edges_src',
                'idx_edges_dst',
                'idx_edges_relation',
                'idx_occurrences_node_role',
                'idx_occurrences_file',
                'idx_edge_evidence_edge'
              )
            "#,
        )
        .fetch_one(&store.pool)
        .await?;
        assert_eq!(index_count, 9);

        let node_search_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM sqlite_master
            WHERE type = 'table'
              AND name = 'node_search'
            "#,
        )
        .fetch_one(&store.pool)
        .await?;
        assert_eq!(node_search_count, 1);

        Ok(())
    }

    #[test]
    fn deterministic_ids_are_stable() {
        let first_node_id = node_id(1, "rust", "file:///demo/src/lib.rs#function:caller:1:0");
        let second_node_id = node_id(1, "rust", "file:///demo/src/lib.rs#function:caller:1:0");
        assert_eq!(first_node_id, second_node_id);

        let first_edge_id = edge_id(1, &first_node_id, "callee", "calls", None);
        let second_edge_id = edge_id(1, &first_node_id, "callee", "calls", None);
        assert_eq!(first_edge_id, second_edge_id);
    }

    #[tokio::test]
    async fn demo_seed_inserts_core_graph_rows() -> std::result::Result<(), Box<dyn Error>> {
        let store = migrated_store().await?;

        store.demo_seed("file:///demo").await?;

        assert_eq!(
            store.stats().await?,
            GraphStoreStats {
                workspaces: 1,
                extraction_runs: 1,
                files: 1,
                nodes: 2,
                edges: 1,
                occurrences: 1,
                edge_evidence: 1,
            }
        );

        Ok(())
    }

    #[tokio::test]
    async fn upserts_do_not_duplicate_canonical_rows() -> std::result::Result<(), Box<dyn Error>> {
        let store = migrated_store().await?;

        let first = store.demo_seed("file:///demo").await?;
        let second = store.demo_seed("file:///demo").await?;

        assert_eq!(first.workspace_id, second.workspace_id);
        assert_eq!(first.file_id, second.file_id);
        assert_eq!(first.caller_node_id, second.caller_node_id);
        assert_eq!(first.callee_node_id, second.callee_node_id);
        assert_eq!(first.edge_id, second.edge_id);
        assert_eq!(
            store.stats().await?,
            GraphStoreStats {
                workspaces: 1,
                extraction_runs: 2,
                files: 1,
                nodes: 2,
                edges: 1,
                occurrences: 2,
                edge_evidence: 2,
            }
        );

        Ok(())
    }

    #[tokio::test]
    async fn foreign_keys_reject_invalid_edge_references() -> std::result::Result<(), Box<dyn Error>>
    {
        let store = migrated_store().await?;
        let workspace_id = store.create_workspace("file:///demo", "rust").await?;

        let error = store
            .upsert_edge(EdgeInput {
                workspace_id,
                src_node_id: "missing-src",
                dst_node_id: "missing-dst",
                relation: "calls",
                context: None,
                confidence: "EXTRACTED",
                confidence_score: 1.0,
                weight: 1.0,
                properties_json: json!({}),
                run_id: None,
            })
            .await;

        assert!(error.is_err());

        Ok(())
    }
}
