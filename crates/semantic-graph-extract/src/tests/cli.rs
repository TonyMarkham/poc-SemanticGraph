use crate::{
    cli::{
        Cli, Command, RustFileMode, resolve_cli_database_path, resolve_rust_file_mode,
        resolve_rust_workspace_routes, validate_deleted_rust_file_request,
    },
    workspace_extraction::WorkspaceExtractionRoutes,
};

use clap::Parser;
use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn rust_file_requires_only_file_and_defaults_workspace_root() -> Result<(), Box<dyn Error>> {
    let cli = Cli::try_parse_from([
        "semantic-graph-extract",
        "rust-file",
        "crates/semantic-graph-extract/src/main.rs",
    ])?;

    match cli.command {
        Command::RustFile {
            db,
            workspace_root,
            calls,
            references,
            symbols,
            file,
        } => {
            assert_eq!(db, None);
            assert_eq!(workspace_root, PathBuf::from("."));
            assert!(!calls);
            assert!(!references);
            assert!(!symbols);
            assert_eq!(
                file,
                PathBuf::from("crates/semantic-graph-extract/src/main.rs")
            );
        }
        _ => return Err("expected rust-file command".into()),
    }

    Ok(())
}

#[test]
fn rust_file_accepts_workspace_root_and_symbols_mode() -> Result<(), Box<dyn Error>> {
    let cli = Cli::try_parse_from([
        "semantic-graph-extract",
        "rust-file",
        "--workspace-root",
        ".",
        "crates/semantic-graph-extract/src/main.rs",
        "--symbols",
    ])?;

    match cli.command {
        Command::RustFile {
            workspace_root,
            symbols,
            file,
            ..
        } => {
            assert_eq!(workspace_root, PathBuf::from("."));
            assert!(symbols);
            assert_eq!(
                file,
                PathBuf::from("crates/semantic-graph-extract/src/main.rs")
            );
        }
        _ => return Err("expected rust-file command".into()),
    }

    Ok(())
}

#[test]
fn rust_file_modes_are_mutually_exclusive() -> Result<(), Box<dyn Error>> {
    assert_eq!(
        resolve_rust_file_mode(false, false, false)?,
        RustFileMode::Full
    );
    assert_eq!(
        resolve_rust_file_mode(false, false, true)?,
        RustFileMode::Symbols
    );
    assert!(resolve_rust_file_mode(true, true, false).is_err());
    Ok(())
}

#[test]
fn rust_file_deleted_defaults_workspace_root() -> Result<(), Box<dyn Error>> {
    let cli = Cli::try_parse_from([
        "semantic-graph-extract",
        "rust-file-deleted",
        "crates/wip/src/foo.rs",
    ])?;

    match cli.command {
        Command::RustFileDeleted {
            db,
            workspace_root,
            file,
        } => {
            assert_eq!(db, None);
            assert_eq!(workspace_root, PathBuf::from("."));
            assert_eq!(file, PathBuf::from("crates/wip/src/foo.rs"));
        }
        _ => return Err("expected rust-file-deleted command".into()),
    }

    Ok(())
}

#[test]
fn rust_crate_defaults_workspace_root_and_routes() -> Result<(), Box<dyn Error>> {
    let cli = Cli::try_parse_from(["semantic-graph-extract", "rust-crate", "crates/wip"])?;

    match cli.command {
        Command::RustCrate {
            db,
            workspace_root,
            analysis_workers,
            calls,
            references,
            symbols,
            package_path,
        } => {
            assert_eq!(db, None);
            assert_eq!(workspace_root, PathBuf::from("."));
            assert_eq!(analysis_workers, None);
            assert!(!calls);
            assert!(!references);
            assert!(!symbols);
            assert_eq!(package_path, PathBuf::from("crates/wip"));
        }
        _ => return Err("expected rust-crate command".into()),
    }

    Ok(())
}

#[test]
fn rust_crate_accepts_analysis_workers_and_combined_routes() -> Result<(), Box<dyn Error>> {
    let cli = Cli::try_parse_from([
        "semantic-graph-extract",
        "rust-crate",
        "--workspace-root",
        ".",
        "--analysis-workers",
        "3",
        "--symbols",
        "--references",
        "crates/wip",
    ])?;

    match cli.command {
        Command::RustCrate {
            workspace_root,
            analysis_workers,
            calls,
            references,
            symbols,
            package_path,
            ..
        } => {
            assert_eq!(workspace_root, PathBuf::from("."));
            assert_eq!(analysis_workers, Some(3));
            assert!(!calls);
            assert!(references);
            assert!(symbols);
            assert_eq!(package_path, PathBuf::from("crates/wip"));
        }
        _ => return Err("expected rust-crate command".into()),
    }

    Ok(())
}

#[test]
fn rust_workspace_defaults_workspace_root_and_routes() -> Result<(), Box<dyn Error>> {
    let cli = Cli::try_parse_from(["semantic-graph-extract", "rust-workspace"])?;

    match cli.command {
        Command::RustWorkspace {
            db,
            workspace_root,
            analysis_workers,
            calls,
            references,
            symbols,
        } => {
            assert_eq!(db, None);
            assert_eq!(workspace_root, PathBuf::from("."));
            assert_eq!(analysis_workers, None);
            assert!(!calls);
            assert!(!references);
            assert!(!symbols);
        }
        _ => return Err("expected rust-workspace command".into()),
    }

    Ok(())
}

#[test]
fn rust_workspace_routes_default_to_all_and_allow_combinations() {
    assert_eq!(
        resolve_rust_workspace_routes(false, false, false),
        WorkspaceExtractionRoutes::all()
    );
    assert_eq!(
        resolve_rust_workspace_routes(true, true, false).label(),
        "references+calls"
    );
    assert_eq!(
        resolve_rust_workspace_routes(false, false, true).label(),
        "symbols"
    );
}

#[test]
fn rust_file_deleted_accepts_missing_file_path() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("deleted-file-root")?;
    let deleted_file = PathBuf::from("crates/wip/src/deleted.rs");
    let (workspace_root, file_uri, relative_path) =
        validate_deleted_rust_file_request(root.clone(), &deleted_file)?;

    assert_eq!(workspace_root, root.canonicalize()?);
    assert!(file_uri.ends_with("/crates/wip/src/deleted.rs"));
    assert_eq!(relative_path, "crates/wip/src/deleted.rs");

    Ok(())
}

#[test]
fn cli_database_path_overrides_config() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("extract-db-overrides-config")?;
    let config_path = write_config(&root, "path = \".local/config.db\"")?;
    let override_path = root.join("scratch.db");

    let resolved =
        resolve_cli_database_path(Some(override_path.clone()), &Some(config_path), &root)?;

    assert_eq!(resolved, override_path);
    Ok(())
}

#[test]
fn cli_database_path_discovers_config_from_workspace_root() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("extract-discovers-config")?;
    let config_path = write_config(&root, "path = \".local/config.db\"")?;
    let workspace_subdirectory = root.join("crates/example");
    fs::create_dir_all(&workspace_subdirectory)?;

    let resolved = resolve_cli_database_path(None, &None, &workspace_subdirectory)?;

    assert_eq!(
        resolved,
        config_path
            .parent()
            .ok_or("expected config parent")?
            .join(".local/config.db")
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
        "semantic-graph-extract-{name}-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir_all(&path)?;
    Ok(path)
}
