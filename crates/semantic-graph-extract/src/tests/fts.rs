use crate::fts::{
    FtsExclusionSet, FtsExtractionOptions, FtsExtractionRunner, FtsFileDiscovery, FtsFileLanguage,
};

use semantic_graph_config::FtsConfig;
use semantic_graph_db_manager::WriteManager;
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
async fn fts_runner_persists_trigram_content_and_closes_stale_documents()
-> Result<(), Box<dyn Error>> {
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

    let db_path = temp.join("fts.db");
    let writer = WriteManager::start(&db_path).await?;
    writer.migrate().await?;
    let summary = FtsExtractionRunner::run(
        &writer,
        &root,
        &FtsConfig::default(),
        FtsExtractionOptions::default(),
    )
    .await?;

    assert_eq!(summary.indexed_files, 4);
    assert_eq!(summary.skipped_binary_or_unreadable, 1);
    assert_eq!(summary.stale_fts_documents_closed, 0);

    let second_summary = FtsExtractionRunner::run(
        &writer,
        &root,
        &FtsConfig::default(),
        FtsExtractionOptions::default(),
    )
    .await?;
    assert_eq!(second_summary.indexed_files, 4);
    assert_eq!(second_summary.stale_fts_documents_closed, 0);

    fs::remove_file(root.join("notes.txt"))?;
    let third_summary = FtsExtractionRunner::run(
        &writer,
        &root,
        &FtsConfig::default(),
        FtsExtractionOptions::default(),
    )
    .await?;
    assert_eq!(third_summary.indexed_files, 3);
    assert_eq!(third_summary.stale_fts_documents_closed, 1);
    writer.shutdown().await?;

    let pool = sqlite_pool(&db_path).await?;
    let file_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files")
        .fetch_one(&pool)
        .await?;
    let active_document_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM fts_documents WHERE valid_to_run_id IS NULL")
            .fetch_one(&pool)
            .await?;
    let stale_document_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM fts_documents WHERE valid_to_run_id IS NOT NULL")
            .fetch_one(&pool)
            .await?;
    let trigram_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM fts_document_trigram_ci")
        .fetch_one(&pool)
        .await?;
    assert_eq!(file_count, 4);
    assert_eq!(active_document_count, 3);
    assert_eq!(stale_document_count, 1);
    assert_eq!(trigram_count, 3);

    let stored_content: String =
        sqlx::query_scalar("SELECT content FROM fts_document_trigram_ci WHERE path = 'README.md'")
            .fetch_one(&pool)
            .await?;
    assert!(stored_content.contains("ExactCaseToken"));

    assert_candidate(&pool, "%foo::bar%").await?;
    assert_candidate(&pool, "%get_user(%").await?;
    assert_candidate(&pool, "%path/to/file%").await?;

    Ok(())
}

async fn assert_candidate(pool: &SqlitePool, pattern: &str) -> Result<(), Box<dyn Error>> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM fts_document_trigram_ci WHERE content LIKE ?")
            .bind(pattern)
            .fetch_one(pool)
            .await?;
    assert_eq!(count, 1);
    Ok(())
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
