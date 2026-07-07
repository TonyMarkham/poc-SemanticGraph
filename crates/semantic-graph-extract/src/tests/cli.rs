use crate::{
    cli::{
        CSharpFileMode, Cli, Command, RustFileMode, SoulFileMode, resolve_cli_database_path,
        resolve_cli_fts_analysis_workers, resolve_cli_fts_database_path,
        resolve_csharp_extractor_plan, resolve_csharp_file_mode, resolve_csharp_workspace_routes,
        resolve_rust_file_mode, resolve_rust_workspace_routes, resolve_solution_from,
        resolve_soul_file_mode, resolve_soul_lsp_config, resolve_soul_workspace_routes,
        validate_deleted_rust_file_request,
    },
    workspace_extraction::WorkspaceExtractionRoutes,
};

use clap::Parser;
use semantic_graph_config::{FtsConfig, ensure_config_with_csharp_defaults, load_config};
use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn progress_defaults_to_disabled() -> Result<(), Box<dyn Error>> {
    let cli = Cli::try_parse_from(["semantic-graph-extract", "fts"])?;

    assert!(!cli.progress);
    Ok(())
}

#[test]
fn progress_is_global_before_subcommand() -> Result<(), Box<dyn Error>> {
    let cli = Cli::try_parse_from(["semantic-graph-extract", "--progress", "rust-workspace"])?;

    assert!(cli.progress);
    match cli.command {
        Command::RustWorkspace { .. } => {}
        _ => return Err("expected rust-workspace command".into()),
    }
    Ok(())
}

#[test]
fn verbose_defaults_to_disabled() -> Result<(), Box<dyn Error>> {
    let cli = Cli::try_parse_from(["semantic-graph-extract", "fts"])?;

    assert!(!cli.verbose);
    Ok(())
}

#[test]
fn verbose_is_global_before_subcommand() -> Result<(), Box<dyn Error>> {
    let cli = Cli::try_parse_from(["semantic-graph-extract", "--verbose", "rust-workspace"])?;

    assert!(cli.verbose);
    match cli.command {
        Command::RustWorkspace { .. } => {}
        _ => return Err("expected rust-workspace command".into()),
    }
    Ok(())
}

#[test]
fn fts_defaults_to_all_text_files() -> Result<(), Box<dyn Error>> {
    let cli = Cli::try_parse_from(["semantic-graph-extract", "fts"])?;

    match cli.command {
        Command::Fts {
            db,
            analysis_workers,
            no_rust,
            no_csharp,
            no_submodules,
        } => {
            assert_eq!(db, None);
            assert_eq!(analysis_workers, None);
            assert!(!no_rust);
            assert!(!no_csharp);
            assert!(!no_submodules);
        }
        _ => return Err("expected fts command".into()),
    }

    Ok(())
}

#[test]
fn fts_accepts_exclusion_flags_and_db() -> Result<(), Box<dyn Error>> {
    let cli = Cli::try_parse_from([
        "semantic-graph-extract",
        "fts",
        "--db",
        "scratch.db",
        "--analysis-workers",
        "3",
        "--no-rust",
        "--no-csharp",
        "--no-submodules",
    ])?;

    match cli.command {
        Command::Fts {
            db,
            analysis_workers,
            no_rust,
            no_csharp,
            no_submodules,
        } => {
            assert_eq!(db, Some(PathBuf::from("scratch.db")));
            assert_eq!(analysis_workers, Some(3));
            assert!(no_rust);
            assert!(no_csharp);
            assert!(no_submodules);
        }
        _ => return Err("expected fts command".into()),
    }

    Ok(())
}

#[test]
fn fts_resolvers_prefer_fts_config_before_global_defaults() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("fts-resolvers-prefer-fts-config")?;
    let config_dir = root.join(".refactor-radar");
    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.toml");
    fs::write(
        &config_path,
        r#"
[database]
path = ".local/graph.db"

[extractor]
analysis_workers = 2

[fts]
db_path = ".refactor-radar/fts.db"
analysis_workers = 7
ignore-directories = []
ignore-files = []
"#,
    )?;
    let fts_config = load_config(&config_path)?.fts().clone();

    let resolved_db =
        resolve_cli_fts_database_path(None, &Some(config_path.clone()), &root, &fts_config)?;
    assert_eq!(resolved_db, root.join(".refactor-radar/fts.db"));

    let cli_db = root.join("cli.db");
    let resolved_cli_db = resolve_cli_fts_database_path(
        Some(cli_db.clone()),
        &Some(config_path.clone()),
        &root,
        &fts_config,
    )?;
    assert_eq!(resolved_cli_db, cli_db);

    assert_eq!(
        resolve_cli_fts_analysis_workers(&None, &root, None, &fts_config)?,
        7
    );
    assert_eq!(
        resolve_cli_fts_analysis_workers(&None, &root, Some(9), &fts_config)?,
        9
    );

    let fallback_config = FtsConfig::default();
    assert_eq!(
        resolve_cli_fts_analysis_workers(
            &Some(config_path.clone()),
            &root,
            None,
            &fallback_config
        )?,
        2
    );
    assert_eq!(
        resolve_cli_fts_database_path(None, &Some(config_path), &root, &fallback_config)?,
        config_dir.join(".local/graph.db")
    );

    Ok(())
}

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
fn soul_file_defaults_workspace_root_and_supports_symbols_mode() -> Result<(), Box<dyn Error>> {
    let cli = Cli::try_parse_from([
        "semantic-graph-extract",
        "soul-file",
        "--workspace-root",
        ".",
        "docs/feature.md",
        "--symbols",
    ])?;

    match cli.command {
        Command::SoulFile {
            db,
            workspace_root,
            references,
            symbols,
            file,
        } => {
            assert_eq!(db, None);
            assert_eq!(workspace_root, PathBuf::from("."));
            assert!(!references);
            assert!(symbols);
            assert_eq!(file, PathBuf::from("docs/feature.md"));
        }
        _ => return Err("expected soul-file command".into()),
    }

    Ok(())
}

#[test]
fn soul_file_modes_are_mutually_exclusive() -> Result<(), Box<dyn Error>> {
    assert_eq!(resolve_soul_file_mode(false, false)?, SoulFileMode::Full);
    assert_eq!(resolve_soul_file_mode(false, true)?, SoulFileMode::Symbols);
    assert_eq!(
        resolve_soul_file_mode(true, false)?,
        SoulFileMode::References
    );
    assert!(resolve_soul_file_mode(true, true).is_err());
    Ok(())
}

#[test]
fn soul_workspace_defaults_to_symbols_and_references_without_calls() {
    let routes = resolve_soul_workspace_routes(false, false);
    assert!(routes.includes_symbols());
    assert!(routes.includes_references());
    assert!(!routes.includes_calls());

    let symbols = resolve_soul_workspace_routes(false, true);
    assert!(symbols.includes_symbols());
    assert!(!symbols.includes_references());
    assert!(!symbols.includes_calls());
}

#[test]
fn soul_lsp_config_resolves_from_refactor_radar_config() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("soul-lsp-config-resolves")?;
    let config_dir = root.join(".refactor-radar");
    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.toml");
    fs::write(
        &config_path,
        r#"
[database]
path = ".local/graph.db"

[soul.scan]
excluded_dirs = [".git", "target", "generated"]
excluded_dir_suffixes = ["Spec"]
excluded_bin_except_under = ["src", "tools"]

[[soul.plugins]]
language = "rust"
path = ".soul/plugins/rust.so"
"#,
    )?;

    let config = resolve_soul_lsp_config(&None, &root)?;

    assert_eq!(
        config.scan().excluded_dirs(),
        &[
            ".git".to_string(),
            "target".to_string(),
            "generated".to_string()
        ]
    );
    assert_eq!(config.scan().excluded_dir_suffixes(), &["Spec".to_string()]);
    assert_eq!(
        config.scan().excluded_bin_except_under(),
        &["src".to_string(), "tools".to_string()]
    );
    assert_eq!(config.plugins().len(), 1);
    assert_eq!(config.plugins()[0].language(), "rust");
    assert_eq!(
        config.plugins()[0].path(),
        &PathBuf::from(".soul/plugins/rust.so")
    );
    Ok(())
}

#[test]
fn csharp_file_defaults_and_accepts_solution() -> Result<(), Box<dyn Error>> {
    let cli = Cli::try_parse_from([
        "semantic-graph-extract",
        "csharp-file",
        "--solution",
        "Demo.slnx",
        "Project/Program.cs",
    ])?;

    match cli.command {
        Command::CSharpFile {
            db,
            solution,
            csharp_ls,
            calls,
            references,
            symbols,
            file,
        } => {
            assert_eq!(db, None);
            assert_eq!(solution, Some(PathBuf::from("Demo.slnx")));
            assert_eq!(csharp_ls, None);
            assert!(!calls);
            assert!(!references);
            assert!(!symbols);
            assert_eq!(file, PathBuf::from("Project/Program.cs"));
        }
        _ => return Err("expected csharp-file command".into()),
    }

    Ok(())
}

#[test]
fn csharp_file_modes_are_mutually_exclusive() -> Result<(), Box<dyn Error>> {
    assert_eq!(
        resolve_csharp_file_mode(false, false, false)?,
        CSharpFileMode::Full
    );
    assert_eq!(
        resolve_csharp_file_mode(false, true, false)?,
        CSharpFileMode::References
    );
    assert!(resolve_csharp_file_mode(true, false, true).is_err());
    Ok(())
}

#[test]
fn csharp_file_deleted_defaults_and_accepts_solution() -> Result<(), Box<dyn Error>> {
    let cli = Cli::try_parse_from([
        "semantic-graph-extract",
        "csharp-file-deleted",
        "--solution",
        "Demo.slnx",
        "Project/Deleted.cs",
    ])?;

    match cli.command {
        Command::CSharpFileDeleted {
            db,
            solution,
            csharp_ls,
            file,
        } => {
            assert_eq!(db, None);
            assert_eq!(solution, Some(PathBuf::from("Demo.slnx")));
            assert_eq!(csharp_ls, None);
            assert_eq!(file, PathBuf::from("Project/Deleted.cs"));
        }
        _ => return Err("expected csharp-file-deleted command".into()),
    }

    Ok(())
}

#[test]
fn csharp_project_accepts_boundary_workers_and_combined_routes() -> Result<(), Box<dyn Error>> {
    let cli = Cli::try_parse_from([
        "semantic-graph-extract",
        "csharp-project",
        "--solution",
        "Demo.slnx",
        "--csharp-ls",
        "/tmp/csharp-ls",
        "--process-workers",
        "2",
        "--symbols",
        "--references",
        "Project/Project.csproj",
    ])?;

    match cli.command {
        Command::CSharpProject {
            db,
            solution,
            csharp_ls,
            process_workers,
            calls,
            references,
            symbols,
            project_or_root,
        } => {
            assert_eq!(db, None);
            assert_eq!(solution, Some(PathBuf::from("Demo.slnx")));
            assert_eq!(csharp_ls, Some(PathBuf::from("/tmp/csharp-ls")));
            assert_eq!(process_workers, Some(2));
            assert!(!calls);
            assert!(references);
            assert!(symbols);
            assert_eq!(project_or_root, PathBuf::from("Project/Project.csproj"));
        }
        _ => return Err("expected csharp-project command".into()),
    }

    Ok(())
}

#[test]
fn csharp_solution_accepts_workers_and_combined_routes() -> Result<(), Box<dyn Error>> {
    let cli = Cli::try_parse_from([
        "semantic-graph-extract",
        "csharp-solution",
        "--solution",
        "Demo.slnx",
        "--process-workers",
        "3",
        "--calls",
        "--references",
    ])?;

    match cli.command {
        Command::CSharpSolution {
            db,
            solution,
            csharp_ls,
            process_workers,
            calls,
            references,
            symbols,
        } => {
            assert_eq!(db, None);
            assert_eq!(solution, Some(PathBuf::from("Demo.slnx")));
            assert_eq!(csharp_ls, None);
            assert_eq!(process_workers, Some(3));
            assert!(calls);
            assert!(references);
            assert!(!symbols);
        }
        _ => return Err("expected csharp-solution command".into()),
    }

    Ok(())
}

#[test]
fn csharp_workspace_routes_default_to_all_and_allow_combinations() {
    assert_eq!(
        resolve_csharp_workspace_routes(false, false, false),
        WorkspaceExtractionRoutes::all()
    );
    assert_eq!(
        resolve_csharp_workspace_routes(true, true, false).label(),
        "references+calls"
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

#[test]
fn csharp_solution_resolution_cli_overrides_config() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("csharp-solution-cli-overrides-config")?;
    let cli_solution = root.join("Cli.slnx");
    let config_solution = root.join("Config.slnx");
    fs::write(&cli_solution, "")?;
    fs::write(&config_solution, "")?;

    let resolved = resolve_solution_from(Some(cli_solution.clone()), Some(config_solution), &root)?;

    assert_eq!(resolved, cli_solution);
    Ok(())
}

#[test]
fn csharp_solution_resolution_config_overrides_discovery() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("csharp-solution-config-overrides-discovery")?;
    let config_solution = root.join("Config.sln");
    let discovered_solution = root.join("Discovered.slnx");
    fs::write(&config_solution, "")?;
    fs::write(&discovered_solution, "")?;

    let resolved = resolve_solution_from(None, Some(config_solution.clone()), &root)?;

    assert_eq!(resolved, config_solution);
    Ok(())
}

#[test]
fn csharp_solution_resolution_discovers_slnx_before_sln() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("csharp-solution-discovers-slnx")?;
    let sln = root.join("A.sln");
    let slnx = root.join("B.slnx");
    fs::write(&sln, "")?;
    fs::write(&slnx, "")?;

    let resolved = resolve_solution_from(None, None, &root)?;

    assert_eq!(resolved, slnx);
    Ok(())
}

#[test]
fn csharp_solution_resolution_errors_when_missing() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("csharp-solution-missing")?;

    let error = resolve_solution_from(None, None, &root)
        .err()
        .ok_or("expected missing solution error")?;

    assert!(error.to_string().contains("pass --solution"));
    Ok(())
}

#[test]
fn csharp_extractor_plan_applies_cli_overrides() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("csharp-plan-cli-overrides")?;
    let config_path = root.join(".refactor-radar/config.toml");
    ensure_config_with_csharp_defaults(&config_path)?;
    let cli_solution = root.join("Cli.slnx");
    fs::write(&cli_solution, "")?;

    let plan = resolve_csharp_extractor_plan(
        &Some(config_path),
        &root,
        Some(PathBuf::from("/tmp/custom-csharp-ls")),
        Some(cli_solution.clone()),
        Some(4),
    )?;

    assert_eq!(plan.binary(), &PathBuf::from("/tmp/custom-csharp-ls"));
    assert_eq!(plan.solution(), &cli_solution);
    assert_eq!(plan.process_workers(), 4);
    assert_eq!(plan.log_level(), "warning");
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
