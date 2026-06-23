use crate::{
    ConfigError, ExtractorMode, LoadOptions, ResolvedDatabasePathSource, discover_config,
    ensure_config_with_csharp_defaults, load_config, resolve_database_path,
};

use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn parses_valid_config() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("parses-valid-config")?;
    let config_path = write_config(&root, "path = \".local/test.db\"")?;

    let config = load_config(&config_path)?;

    assert_eq!(config.database().path(), &PathBuf::from(".local/test.db"));
    assert_eq!(config.extractor().mode(), ExtractorMode::Serial);
    assert_eq!(config.extractor().jobs(), None);
    assert_eq!(config.extractor().reference_jobs(), None);
    assert_eq!(config.extractor().call_jobs(), None);
    assert_eq!(config.extractor().analysis_workers(), None);
    assert_eq!(config.extractor().reference_analysis_workers(), None);
    assert_eq!(config.extractor().call_analysis_workers(), None);
    assert_eq!(config.writer().queue_capacity(), 4096);
    assert_eq!(config.writer().max_rows_per_commit(), 1000);
    assert_eq!(config.writer().max_millis_per_commit(), 250);
    assert_eq!(config.writer().busy_timeout_ms(), 5000);
    assert_eq!(config.query_service().latest_run_limit(), 10);
    assert_eq!(config.query_service().max_search_limit(), 50);
    assert_eq!(config.query_service().max_projection_limit(), 1000);
    assert_eq!(config.query_service().max_neighbors_limit(), 100);
    assert_eq!(config.query_service().max_file_edge_limit(), 200);
    assert_eq!(config.query_service().max_route_status_limit(), 200);
    assert_eq!(config.query_service().max_shortest_path_depth(), 12);
    assert_eq!(config.query_service().max_shortest_path_visited(), 5000);
    assert_eq!(config.csharp().binary(), &PathBuf::from("csharp-ls"));
    assert_eq!(config.csharp().solution(), None);
    assert_eq!(config.csharp().log_level(), "warning");
    assert_eq!(config.csharp().features(), &[] as &[String]);
    assert_eq!(config.csharp().analysis_workers(), 1);
    assert_eq!(config.csharp().startup_timeout_ms(), 120000);
    assert_eq!(config.csharp().request_timeout_ms(), 30000);
    assert_eq!(
        config.soul().scan().excluded_dirs(),
        &[
            ".git".to_string(),
            ".soul".to_string(),
            "target".to_string(),
            ".idea".to_string(),
            ".vscode".to_string(),
            ".vs".to_string(),
            ".codex".to_string(),
            "node_modules".to_string(),
            "obj".to_string(),
        ]
    );
    assert_eq!(
        config.soul().scan().excluded_dir_suffixes(),
        &[
            "Tests".to_string(),
            ".Tests".to_string(),
            "tests".to_string(),
            ".tests".to_string(),
        ]
    );
    assert_eq!(
        config.soul().scan().excluded_bin_except_under(),
        &["src".to_string()]
    );
    assert!(config.soul().plugins().is_empty());
    assert_eq!(config.fts().db_path(), None);
    assert_eq!(config.fts().analysis_workers(), None);
    assert_eq!(config.fts().max_indexed_file_bytes(), 209715200);
    assert_eq!(config.fts().ignore_directories(), &[] as &[String]);
    assert_eq!(config.fts().ignore_files(), &[] as &[String]);
    Ok(())
}

#[test]
fn parses_extractor_config() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("parses-extractor-config")?;
    let config_dir = root.join(".refactor-radar");
    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.toml");
    fs::write(
        &config_path,
        r#"
[database]
path = ".local/test.db"

[extractor]
mode = "threaded"
jobs = 8
reference_jobs = 5
call_jobs = 3
analysis_workers = 2
reference_analysis_workers = 4
call_analysis_workers = 0
"#,
    )?;

    let config = load_config(&config_path)?;

    assert_eq!(config.extractor().mode(), ExtractorMode::Threaded);
    assert_eq!(config.extractor().jobs(), Some(8));
    assert_eq!(config.extractor().reference_jobs(), Some(5));
    assert_eq!(config.extractor().call_jobs(), Some(3));
    assert_eq!(config.extractor().analysis_workers(), Some(2));
    assert_eq!(config.extractor().reference_analysis_workers(), Some(4));
    assert_eq!(config.extractor().call_analysis_workers(), Some(0));
    Ok(())
}

#[test]
fn rejects_invalid_extractor_config() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("rejects-invalid-extractor-config")?;
    let config_dir = root.join(".refactor-radar");
    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.toml");
    fs::write(
        &config_path,
        r#"
[database]
path = ".local/test.db"

[extractor]
mode = "parallel"
"#,
    )?;

    let error = load_config(&config_path)
        .err()
        .ok_or("expected config error")?;

    assert!(matches!(error, ConfigError::InvalidExtractorSetting { .. }));
    Ok(())
}

#[test]
fn rejects_invalid_analysis_worker_config() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("rejects-invalid-analysis-worker-config")?;
    let config_dir = root.join(".refactor-radar");
    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.toml");
    fs::write(
        &config_path,
        r#"
[database]
path = ".local/test.db"

[extractor]
analysis_workers = 0
"#,
    )?;

    let error = load_config(&config_path)
        .err()
        .ok_or("expected config error")?;

    assert!(matches!(error, ConfigError::InvalidExtractorSetting { .. }));
    Ok(())
}

#[test]
fn parses_writer_config() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("parses-writer-config")?;
    let config_dir = root.join(".refactor-radar");
    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.toml");
    fs::write(
        &config_path,
        r#"
[database]
path = ".local/test.db"

[writer]
queue_capacity = 128
max_rows_per_commit = 64
max_millis_per_commit = 50
busy_timeout_ms = 2500
"#,
    )?;

    let config = load_config(&config_path)?;

    assert_eq!(config.writer().queue_capacity(), 128);
    assert_eq!(config.writer().max_rows_per_commit(), 64);
    assert_eq!(config.writer().max_millis_per_commit(), 50);
    assert_eq!(config.writer().busy_timeout_ms(), 2500);
    Ok(())
}

#[test]
fn parses_query_service_config() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("parses-query-service-config")?;
    let config_dir = root.join(".refactor-radar");
    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.toml");
    fs::write(
        &config_path,
        r#"
[database]
path = ".local/test.db"

[query-service]
latest_run_limit = 3
max_search_limit = 4
max_projection_limit = 5
max_neighbors_limit = 6
max_file_edge_limit = 7
max_route_status_limit = 8
max_shortest_path_depth = 9
max_shortest_path_visited = 10
"#,
    )?;

    let config = load_config(&config_path)?;

    assert_eq!(config.query_service().latest_run_limit(), 3);
    assert_eq!(config.query_service().max_search_limit(), 4);
    assert_eq!(config.query_service().max_projection_limit(), 5);
    assert_eq!(config.query_service().max_neighbors_limit(), 6);
    assert_eq!(config.query_service().max_file_edge_limit(), 7);
    assert_eq!(config.query_service().max_route_status_limit(), 8);
    assert_eq!(config.query_service().max_shortest_path_depth(), 9);
    assert_eq!(config.query_service().max_shortest_path_visited(), 10);
    Ok(())
}

#[test]
fn rejects_invalid_query_service_config() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("rejects-invalid-query-service-config")?;
    let config_dir = root.join(".refactor-radar");
    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.toml");
    fs::write(
        &config_path,
        r#"
[database]
path = ".local/test.db"

[query-service]
max_search_limit = 0
"#,
    )?;

    let error = load_config(&config_path)
        .err()
        .ok_or("expected config error")?;

    assert!(matches!(
        error,
        ConfigError::InvalidQueryServiceSetting { .. }
    ));
    Ok(())
}

#[test]
fn parses_csharp_config() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("parses-csharp-config")?;
    let config_dir = root.join(".refactor-radar");
    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.toml");
    fs::write(
        &config_path,
        r#"
[database]
path = ".local/test.db"

[csharp]
binary = "/tmp/csharp-ls"
solution = "Demo.slnx"
log_level = "debug"
features = ["metadata-uris"]
analysis_workers = 3
startup_timeout_ms = 5000
request_timeout_ms = 1000
"#,
    )?;

    let config = load_config(&config_path)?;

    assert_eq!(config.csharp().binary(), &PathBuf::from("/tmp/csharp-ls"));
    assert_eq!(
        config.csharp().solution(),
        Some(&PathBuf::from("Demo.slnx"))
    );
    assert_eq!(config.csharp().log_level(), "debug");
    assert_eq!(config.csharp().features(), &["metadata-uris".to_string()]);
    assert_eq!(config.csharp().analysis_workers(), 3);
    assert_eq!(config.csharp().startup_timeout_ms(), 5000);
    assert_eq!(config.csharp().request_timeout_ms(), 1000);
    Ok(())
}

#[test]
fn rejects_invalid_csharp_analysis_workers() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("rejects-invalid-csharp-workers")?;
    let config_dir = root.join(".refactor-radar");
    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.toml");
    fs::write(
        &config_path,
        r#"
[database]
path = ".local/test.db"

[csharp]
analysis_workers = 0
"#,
    )?;

    let error = load_config(&config_path)
        .err()
        .ok_or("expected config error")?;

    assert!(matches!(error, ConfigError::InvalidCSharpSetting { .. }));
    Ok(())
}

#[test]
fn rejects_invalid_csharp_timeouts() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("rejects-invalid-csharp-timeouts")?;
    let config_dir = root.join(".refactor-radar");
    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.toml");
    fs::write(
        &config_path,
        r#"
[database]
path = ".local/test.db"

[csharp]
startup_timeout_ms = 0
"#,
    )?;

    let error = load_config(&config_path)
        .err()
        .ok_or("expected config error")?;

    assert!(matches!(error, ConfigError::InvalidCSharpSetting { .. }));
    Ok(())
}

#[test]
fn ignores_extractor_workers_for_csharp_defaults() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("ignores-extractor-workers-for-csharp")?;
    let config_dir = root.join(".refactor-radar");
    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.toml");
    fs::write(
        &config_path,
        r#"
[database]
path = ".local/test.db"

[extractor]
analysis_workers = 7
"#,
    )?;

    let config = load_config(&config_path)?;

    assert_eq!(config.extractor().analysis_workers(), Some(7));
    assert_eq!(config.csharp().analysis_workers(), 1);
    Ok(())
}

#[test]
fn parses_soul_config() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("parses-soul-config")?;
    let config_dir = root.join(".refactor-radar");
    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.toml");
    fs::write(
        &config_path,
        r#"
[database]
path = ".local/test.db"

[soul.scan]
excluded_dirs = [".git", "custom"]
excluded_dir_suffixes = ["Spec"]
excluded_bin_except_under = ["src", "tools"]

[[soul.plugins]]
language = "rust"
path = ".soul/plugins/rust.so"
"#,
    )?;

    let config = load_config(&config_path)?;

    assert_eq!(
        config.soul().scan().excluded_dirs(),
        &[".git".to_string(), "custom".to_string()]
    );
    assert_eq!(
        config.soul().scan().excluded_dir_suffixes(),
        &["Spec".to_string()]
    );
    assert_eq!(
        config.soul().scan().excluded_bin_except_under(),
        &["src".to_string(), "tools".to_string()]
    );
    assert_eq!(config.soul().plugins().len(), 1);
    assert_eq!(config.soul().plugins()[0].language(), "rust");
    assert_eq!(
        config.soul().plugins()[0].path(),
        &PathBuf::from(".soul/plugins/rust.so")
    );
    Ok(())
}

#[test]
fn rejects_invalid_soul_config() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("rejects-invalid-soul-config")?;
    let config_dir = root.join(".refactor-radar");
    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.toml");
    fs::write(
        &config_path,
        r#"
[database]
path = ".local/test.db"

[soul.scan]
excluded_dirs = [""]
"#,
    )?;

    let error = load_config(&config_path)
        .err()
        .ok_or("expected config error")?;

    assert!(matches!(error, ConfigError::InvalidSoulSetting { .. }));

    fs::write(
        &config_path,
        r#"
[database]
path = ".local/test.db"

[[soul.plugins]]
language = ""
path = ".soul/plugins/rust.so"
"#,
    )?;

    let error = load_config(&config_path)
        .err()
        .ok_or("expected config error")?;

    assert!(matches!(error, ConfigError::InvalidSoulSetting { .. }));
    Ok(())
}

#[test]
fn parses_fts_config() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("parses-fts-config")?;
    let config_dir = root.join(".refactor-radar");
    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.toml");
    fs::write(
        &config_path,
        r#"
[database]
path = ".local/test.db"

[fts]
db_path = ".refactor-radar/fts.db"
analysis_workers = 6
max_indexed_file_bytes = 123456
ignore-directories = ["target", "submodules\\graphify", "target"]
ignore-files = ["README.md", "docs\\plan.md"]
"#,
    )?;

    let config = load_config(&config_path)?;

    assert_eq!(
        config.fts().db_path(),
        Some(&PathBuf::from(".refactor-radar/fts.db"))
    );
    assert_eq!(config.fts().analysis_workers(), Some(6));
    assert_eq!(config.fts().max_indexed_file_bytes(), 123456);
    assert_eq!(
        config.fts().ignore_directories(),
        &["submodules/graphify".to_string(), "target".to_string()]
    );
    assert_eq!(
        config.fts().ignore_files(),
        &["README.md".to_string(), "docs/plan.md".to_string()]
    );
    Ok(())
}

#[test]
fn rejects_invalid_fts_limits() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("rejects-invalid-fts-limits")?;
    let config_dir = root.join(".refactor-radar");
    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.toml");
    fs::write(
        &config_path,
        r#"
[database]
path = ".local/test.db"

[fts]
analysis_workers = 0
"#,
    )?;

    let error = load_config(&config_path)
        .err()
        .ok_or("expected config error")?;

    assert!(matches!(error, ConfigError::InvalidFtsSetting { .. }));

    fs::write(
        &config_path,
        r#"
[database]
path = ".local/test.db"

[fts]
max_indexed_file_bytes = 0
"#,
    )?;

    let error = load_config(&config_path)
        .err()
        .ok_or("expected config error")?;

    assert!(matches!(error, ConfigError::InvalidFtsSetting { .. }));
    Ok(())
}

#[test]
fn rejects_invalid_fts_paths() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("rejects-invalid-fts-paths")?;
    let config_dir = root.join(".refactor-radar");
    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.toml");
    fs::write(
        &config_path,
        r#"
[database]
path = ".local/test.db"

[fts]
ignore-directories = ["../outside"]
"#,
    )?;

    let error = load_config(&config_path)
        .err()
        .ok_or("expected config error")?;

    assert!(matches!(error, ConfigError::InvalidFtsSetting { .. }));
    Ok(())
}

#[test]
fn rejects_invalid_writer_config() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("rejects-invalid-writer-config")?;
    let config_dir = root.join(".refactor-radar");
    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.toml");
    fs::write(
        &config_path,
        r#"
[database]
path = ".local/test.db"

[writer]
queue_capacity = 0
"#,
    )?;

    let error = load_config(&config_path)
        .err()
        .ok_or("expected config error")?;

    assert!(matches!(error, ConfigError::InvalidWriterSetting { .. }));
    Ok(())
}

#[test]
fn rejects_missing_database_path() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("rejects-missing-database-path")?;
    let config_dir = root.join(".refactor-radar");
    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.toml");
    fs::write(&config_path, "[database]\n")?;

    let error = load_config(&config_path)
        .err()
        .ok_or("expected config error")?;

    assert!(matches!(error, ConfigError::MissingDatabasePath { .. }));
    Ok(())
}

#[test]
fn resolves_relative_database_path_from_config_directory() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("relative-database-path")?;
    let config_path = write_config(&root, "path = \".local/test.db\"")?;

    let resolved = resolve_database_path(LoadOptions {
        explicit_config_path: Some(config_path.clone()),
        ..LoadOptions::default()
    })?;

    assert_eq!(
        resolved.path(),
        config_path
            .parent()
            .ok_or("expected config parent")?
            .join(".local/test.db")
    );
    assert_eq!(
        resolved.source(),
        ResolvedDatabasePathSource::ExplicitConfig
    );
    Ok(())
}

#[test]
fn preserves_absolute_database_path() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("absolute-database-path")?;
    let absolute_database_path = root.join("absolute.db");
    let config_path = write_config(
        &root,
        &format!("path = \"{}\"", toml_escape(&absolute_database_path)),
    )?;

    let resolved = resolve_database_path(LoadOptions {
        explicit_config_path: Some(config_path),
        ..LoadOptions::default()
    })?;

    assert_eq!(resolved.path(), absolute_database_path);
    Ok(())
}

#[test]
fn discovers_config_from_workspace_subdirectory() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("discovers-config")?;
    let config_path = write_config(&root, "path = \".local/test.db\"")?;
    let subdirectory = root.join("crates/example/src");
    fs::create_dir_all(&subdirectory)?;

    let discovered = discover_config(&subdirectory)?.ok_or("expected discovered config")?;

    assert_eq!(discovered, config_path);
    Ok(())
}

#[test]
fn returns_none_when_discovery_reaches_filesystem_root() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("discovers-none")?;
    let subdirectory = root.join("crates/example/src");
    fs::create_dir_all(&subdirectory)?;

    let discovered = discover_config(&subdirectory)?;

    assert_eq!(discovered, None);
    Ok(())
}

#[test]
fn explicit_database_path_overrides_config() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("db-overrides-config")?;
    let config_path = write_config(&root, "path = \".local/config.db\"")?;
    let override_path = root.join("scratch.db");

    let resolved = resolve_database_path(LoadOptions {
        explicit_database_path: Some(override_path.clone()),
        explicit_config_path: Some(config_path),
        ..LoadOptions::default()
    })?;

    assert_eq!(resolved.path(), override_path);
    assert_eq!(
        resolved.source(),
        ResolvedDatabasePathSource::ExplicitDatabasePath
    );
    Ok(())
}

#[test]
fn missing_config_and_database_path_returns_typed_error() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("missing-db-path")?;

    let error = resolve_database_path(LoadOptions {
        discovery_start_dir: Some(root),
        ..LoadOptions::default()
    })
    .err()
    .ok_or("expected missing database path error")?;

    assert!(matches!(error, ConfigError::MissingDatabasePath { .. }));
    Ok(())
}

#[test]
fn creates_default_config_template_when_missing() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("creates-default-config-template")?;
    let config_path = root.join(".refactor-radar/config.toml");

    ensure_config_with_csharp_defaults(&config_path)?;

    let contents = fs::read_to_string(&config_path)?;
    assert!(contents.contains("[database]"));
    assert!(contents.contains("[writer]"));
    assert!(contents.contains("[query-service]"));
    assert!(contents.contains("[fts]"));
    assert!(contents.contains("db_path = \".refactor-radar/fts.db\""));
    assert!(contents.contains("analysis_workers = 8"));
    assert!(contents.contains("max_indexed_file_bytes = 209715200"));
    assert!(contents.contains("[csharp]"));
    assert!(contents.contains("[soul.scan]"));
    assert!(contents.contains("ignore-directories = []"));
    assert!(contents.contains("ignore-files = []"));
    assert!(contents.contains("max_shortest_path_visited = 5000"));
    assert!(contents.contains("solution = \"SemanticGraph.Visualizer.slnx\""));
    assert!(contents.contains("excluded_dirs = [\".git\", \".soul\", \"target\""));
    assert!(contents.contains("excluded_bin_except_under = [\"src\"]"));
    assert!(contents.contains("[[soul.plugins]]"));
    assert!(contents.contains(&format!(
        "path = \"./.soul/plugins/rust{}\"",
        std::env::consts::DLL_SUFFIX
    )));
    assert!(contents.contains(&format!(
        "path = \"./.soul/plugins/csharp{}\"",
        std::env::consts::DLL_SUFFIX
    )));

    let config = load_config(&config_path)?;
    assert_eq!(
        config.csharp().solution(),
        Some(&PathBuf::from("SemanticGraph.Visualizer.slnx"))
    );
    assert_eq!(config.soul().plugins().len(), 2);
    Ok(())
}

#[test]
fn adds_missing_csharp_table_without_changing_existing_values() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("adds-missing-csharp-table")?;
    let config_path = write_config(&root, "path = \".local/custom.db\"")?;

    ensure_config_with_csharp_defaults(&config_path)?;

    let contents = fs::read_to_string(&config_path)?;
    assert!(contents.contains("path = \".local/custom.db\""));
    assert!(contents.contains("[fts]"));
    assert!(contents.contains("ignore-directories = []"));
    assert!(contents.contains("[csharp]"));
    assert!(contents.contains("binary = \"csharp-ls\""));
    assert!(contents.contains("request_timeout_ms = 30000"));
    assert!(contents.contains("[soul.scan]"));
    assert!(contents.contains("excluded_dir_suffixes = [\"Tests\", \".Tests\""));
    assert!(contents.contains("[[soul.plugins]]"));
    assert!(contents.contains(&format!(
        "path = \"./.soul/plugins/rust{}\"",
        std::env::consts::DLL_SUFFIX
    )));
    Ok(())
}

#[test]
fn adds_missing_csharp_keys_without_overwriting_existing_values() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("adds-missing-csharp-keys")?;
    let config_dir = root.join(".refactor-radar");
    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.toml");
    fs::write(
        &config_path,
        r#"
[database]
path = ".local/custom.db"

[csharp]
binary = "/custom/csharp-ls"
analysis_workers = 5
"#,
    )?;

    ensure_config_with_csharp_defaults(&config_path)?;

    let contents = fs::read_to_string(&config_path)?;
    assert!(contents.contains("binary = \"/custom/csharp-ls\""));
    assert!(contents.contains("analysis_workers = 5"));
    assert!(contents.contains("solution = \"SemanticGraph.Visualizer.slnx\""));
    assert!(contents.contains("startup_timeout_ms = 120000"));

    let config = load_config(&config_path)?;
    assert_eq!(
        config.csharp().binary(),
        &PathBuf::from("/custom/csharp-ls")
    );
    assert_eq!(config.csharp().analysis_workers(), 5);
    Ok(())
}

#[test]
fn adds_missing_soul_scan_without_overwriting_existing_plugins() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("keeps-existing-soul-plugins")?;
    let config_dir = root.join(".refactor-radar");
    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.toml");
    fs::write(
        &config_path,
        r#"
[database]
path = ".local/custom.db"

[[soul.plugins]]
language = "rust"
path = "custom/rust.so"
"#,
    )?;

    ensure_config_with_csharp_defaults(&config_path)?;

    let contents = fs::read_to_string(&config_path)?;
    assert!(contents.contains("[soul.scan]"));
    assert!(contents.contains("path = \"custom/rust.so\""));
    assert!(!contents.contains("path = \"./.soul/plugins/csharp"));

    let config = load_config(&config_path)?;
    assert_eq!(config.soul().plugins().len(), 1);
    assert_eq!(
        config.soul().plugins()[0].path(),
        &PathBuf::from("custom/rust.so")
    );
    Ok(())
}

fn write_config(root: &Path, database_line: &str) -> Result<PathBuf, Box<dyn Error>> {
    let config_dir = root.join(".refactor-radar");
    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.toml");
    fs::write(&config_path, format!("[database]\n{database_line}\n"))?;
    Ok(config_path)
}

fn temp_dir(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let path = std::env::temp_dir().join(format!(
        "semantic-graph-config-{name}-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn toml_escape(path: &Path) -> String {
    path.display().to_string().replace('\\', "\\\\")
}
