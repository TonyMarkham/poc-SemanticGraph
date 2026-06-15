use crate::{
    ConfigError, LoadOptions, ResolvedDatabasePathSource, discover_config, load_config,
    resolve_database_path,
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
