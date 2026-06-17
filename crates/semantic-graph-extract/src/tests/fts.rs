use crate::{
    benchmark::BenchmarkSummary,
    fts::{
        FtsExclusionSet, FtsExtractionOptions, FtsExtractionRunner, FtsFileDiscovery,
        FtsFileLanguage,
    },
};

use semantic_graph_config::FtsConfig;
use semantic_graph_db_manager::WriteManager;
use semantic_graph_search_tantivy::TantivyFtsIndex;
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn fts_discovery_honors_config_and_flags() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("fts-discovery")?;
    write_file(&root.join("src/lib.rs"), "fn main() {}\n")?;
    write_file(&root.join("Project/Program.cs"), "class Program {}\n")?;
    write_file(&root.join("README.md"), "# Readme\n")?;
    write_file(&root.join("docs/note.txt"), "note\n")?;
    write_file(&root.join("target/generated.rs"), "fn generated() {}\n")?;
    write_file(&root.join("keep/skip.cs"), "class Skip {}\n")?;
    write_file(&root.join("submod/nested.rs"), "fn nested() {}\n")?;
    write_file(
        &root.join(".gitmodules"),
        "[submodule \"submod\"]\n\tpath = submod\n\turl = ../submod.git\n",
    )?;
    let config = FtsConfig::new(vec!["target".to_string()], vec!["keep/skip.cs".to_string()])?;

    let exclusions = FtsExclusionSet::new(&root, &config, FtsExtractionOptions::default())?;
    let result = FtsFileDiscovery::discover(&root, &exclusions)?;
    let paths = result
        .files()
        .iter()
        .map(|file| file.relative_path().to_string())
        .collect::<Vec<_>>();
    assert!(paths.contains(&"src/lib.rs".to_string()));
    assert!(paths.contains(&"Project/Program.cs".to_string()));
    assert!(paths.contains(&"README.md".to_string()));
    assert!(paths.contains(&"docs/note.txt".to_string()));
    assert!(paths.contains(&"submod/nested.rs".to_string()));
    assert!(!paths.contains(&"target/generated.rs".to_string()));
    assert!(!paths.contains(&"keep/skip.cs".to_string()));
    assert_eq!(result.skipped_by_config(), 2);

    let exclusions =
        FtsExclusionSet::new(&root, &config, FtsExtractionOptions::new(true, true, true))?;
    let result = FtsFileDiscovery::discover(&root, &exclusions)?;
    let paths = result
        .files()
        .iter()
        .map(|file| file.relative_path().to_string())
        .collect::<Vec<_>>();
    assert!(!paths.contains(&"src/lib.rs".to_string()));
    assert!(!paths.contains(&"Project/Program.cs".to_string()));
    assert!(!paths.contains(&"submod/nested.rs".to_string()));
    assert!(paths.contains(&"README.md".to_string()));
    assert!(paths.contains(&"docs/note.txt".to_string()));
    assert_eq!(result.skipped_by_no_rust(), 1);
    assert_eq!(result.skipped_by_no_csharp(), 1);
    assert_eq!(result.skipped_by_no_submodules(), 1);
    Ok(())
}

#[test]
fn fts_language_classification_covers_required_languages() {
    assert_eq!(
        FtsFileLanguage::from_path(Path::new("lib.rs")),
        FtsFileLanguage::Rust
    );
    assert_eq!(
        FtsFileLanguage::from_path(Path::new("Program.cs")),
        FtsFileLanguage::CSharp
    );
    assert_eq!(
        FtsFileLanguage::from_path(Path::new("README.md")),
        FtsFileLanguage::Markdown
    );
    assert_eq!(
        FtsFileLanguage::from_path(Path::new("notes.txt")),
        FtsFileLanguage::Other
    );
}

#[tokio::test]
async fn fts_runner_persists_content_without_fts5_and_updates_sidecar() -> Result<(), Box<dyn Error>>
{
    let temp = temp_dir("fts-runner")?;
    let root = temp.join("workspace");
    fs::create_dir_all(&root)?;
    write_file(
        &root.join("src/lib.rs"),
        "fn get_user() { let path = \"path/to/file\"; foo::bar(); }\n",
    )?;
    write_file(&root.join("Project/Program.cs"), "class Program {}\n")?;
    write_file(&root.join("README.md"), "ExactCaseToken\n")?;
    write_file(&root.join("notes.txt"), "plain text\n")?;
    write_bytes(&root.join("binary.bin"), &[0, 159, 146, 150])?;

    let db_path = root.join(".refactor-radar/fts.db");
    let index_path = root.join(".refactor-radar/fts.tantivy");
    let writer = WriteManager::start(&db_path).await?;
    writer.migrate().await?;
    let summary = FtsExtractionRunner::run(
        &writer,
        &root,
        &db_path,
        &index_path,
        &FtsConfig::default(),
        FtsExtractionOptions::default(),
        2,
    )
    .await?;

    assert_eq!(summary.indexed_files, 4);
    assert_eq!(summary.files_hashed, 4);
    assert_eq!(summary.files_hash_unchanged, 0);
    assert_eq!(summary.files_changed, 4);
    assert_eq!(summary.skipped_binary_or_unreadable, 1);
    assert_eq!(summary.stale_fts_documents_closed, 0);
    assert_benchmark_line(&summary.benchmark, "bench.fts.analysis_workers=2");
    assert_benchmark_line(&summary.benchmark, "bench.fts.discovered_files=5");
    assert_benchmark_line(&summary.benchmark, "bench.fts.file_workers=2");
    assert_benchmark_line(&summary.benchmark, "bench.fts.files_changed=4");
    assert_benchmark_line(&summary.benchmark, "bench.fts.files_hash_unchanged=0");
    assert_benchmark_line(&summary.benchmark, "bench.fts.indexed_files=4");
    assert_benchmark_line(&summary.benchmark, "bench.fts.indexed_documents=4");
    assert_benchmark_line(&summary.benchmark, "bench.fts.skipped_files=4");
    assert_benchmark_line(
        &summary.benchmark,
        "bench.fts.skipped_binary_or_unreadable=1",
    );
    assert_benchmark_line(&summary.benchmark, "bench.fts.write_mode=fts_write_batch");
    assert_benchmark_prefix(&summary.benchmark, "bench.fts.indexed_bytes=");
    assert_benchmark_prefix(&summary.benchmark, "bench.fts.index_update_ms=");
    assert_benchmark_prefix(&summary.benchmark, "bench.fts.total_ms=");

    let second_summary = FtsExtractionRunner::run(
        &writer,
        &root,
        &db_path,
        &index_path,
        &FtsConfig::default(),
        FtsExtractionOptions::default(),
        2,
    )
    .await?;
    assert_eq!(second_summary.indexed_files, 0);
    assert_eq!(second_summary.files_hashed, 4);
    assert_eq!(second_summary.files_hash_unchanged, 4);
    assert_eq!(second_summary.files_changed, 0);
    assert_eq!(second_summary.stale_fts_documents_closed, 0);
    assert_benchmark_line(&second_summary.benchmark, "bench.fts.index_committed=false");

    fs::remove_file(root.join("notes.txt"))?;
    let third_summary = FtsExtractionRunner::run(
        &writer,
        &root,
        &db_path,
        &index_path,
        &FtsConfig::default(),
        FtsExtractionOptions::default(),
        2,
    )
    .await?;
    assert_eq!(third_summary.indexed_files, 0);
    assert_eq!(third_summary.files_hashed, 3);
    assert_eq!(third_summary.files_hash_unchanged, 3);
    assert_eq!(third_summary.files_changed, 0);
    assert_eq!(third_summary.stale_fts_documents_closed, 1);
    writer.shutdown().await?;

    let pool = sqlite_pool(&db_path).await?;
    let active_document_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM fts_documents WHERE valid_to_run_id IS NULL")
            .fetch_one(&pool)
            .await?;
    let content_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM fts_document_contents")
        .fetch_one(&pool)
        .await?;
    assert_eq!(active_document_count, 3);
    assert_eq!(content_count, 3);

    let stored_content: String =
        sqlx::query_scalar("SELECT content FROM fts_document_contents WHERE path = 'README.md'")
            .fetch_one(&pool)
            .await?;
    assert!(stored_content.contains("ExactCaseToken"));
    let route_tag: String = sqlx::query_scalar(
        "SELECT fts_documents.properties_json FROM fts_documents JOIN files ON files.id = fts_documents.file_id WHERE files.path = 'README.md'",
    )
    .fetch_one(&pool)
    .await?;
    assert!(route_tag.contains("fts.full_text"));
    let artifact_document_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM files WHERE path LIKE '.refactor-radar/%'")
            .fetch_one(&pool)
            .await?;
    assert_eq!(artifact_document_count, 0);

    let index = TantivyFtsIndex::open_or_create(&index_path)?;
    assert_eq!(
        index.count_case_insensitive_candidates("exactcasetoken")?,
        1
    );

    Ok(())
}

fn assert_benchmark_line(summary: &BenchmarkSummary, expected: &str) {
    let lines = summary.lines();
    assert!(
        lines.iter().any(|line| line == expected),
        "missing benchmark line {expected}; actual lines: {lines:?}"
    );
}

fn assert_benchmark_prefix(summary: &BenchmarkSummary, expected_prefix: &str) {
    let lines = summary.lines();
    assert!(
        lines.iter().any(|line| line.starts_with(expected_prefix)),
        "missing benchmark line with prefix {expected_prefix}; actual lines: {lines:?}"
    );
}

async fn sqlite_pool(path: &Path) -> Result<SqlitePool, Box<dyn Error>> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(false)
        .foreign_keys(true);
    Ok(SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?)
}

fn write_file(path: &Path, contents: &str) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    Ok(())
}

fn write_bytes(path: &Path, contents: &[u8]) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    Ok(())
}

fn temp_dir(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let path = std::env::temp_dir().join(format!(
        "semantic-graph-extract-{name}-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir_all(&path)?;
    Ok(path)
}
