#![allow(clippy::module_inception)]

mod cli;
mod command;
mod csharp_file_extractions;
mod csharp_file_mode;
mod resolved_csharp_extractor_plan;
mod rust_file_extractions;
mod rust_file_mode;
mod soul_file_extractions;
mod soul_file_mode;

// ---------------------------------------------------------------------------------------------- //

pub use cli::Cli;
pub use command::Command;
pub use csharp_file_extractions::CSharpFileExtractions;
pub use csharp_file_mode::CSharpFileMode;
pub use resolved_csharp_extractor_plan::ResolvedCSharpExtractorPlan;
pub use rust_file_extractions::RustFileExtractions;
pub use rust_file_mode::RustFileMode;
pub use soul_file_extractions::SoulFileExtractions;
pub use soul_file_mode::SoulFileMode;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    ExtractError, ExtractResult,
    document_symbols::paths::{file_uri, workspace_relative_path},
    workspace_extraction::WorkspaceExtractionRoutes,
};

use semantic_graph_config::{
    CSharpConfig, FtsConfig, LoadOptions, SoulConfig, discover_config,
    ensure_config_with_csharp_defaults, load_config, resolve_database_path,
};

use std::{
    env, fs,
    path::{Component, Path, PathBuf},
};

pub fn resolve_cli_database_path(
    db: Option<PathBuf>,
    config: &Option<PathBuf>,
    workspace_root: &Path,
) -> ExtractResult<PathBuf> {
    resolve_database_path(LoadOptions {
        explicit_database_path: db,
        explicit_config_path: config.clone(),
        discovery_start_dir: Some(workspace_root.to_path_buf()),
        default_database_path: None,
    })
    .map(|resolved| resolved.into_path())
    .map_err(ExtractError::config)
}

pub fn resolve_cli_fts_database_path(
    db: Option<PathBuf>,
    config: &Option<PathBuf>,
    workspace_root: &Path,
    fts_config: &FtsConfig,
) -> ExtractResult<PathBuf> {
    if db.is_some() {
        return resolve_cli_database_path(db, config, workspace_root);
    }

    let Some(fts_db_path) = fts_config.db_path() else {
        return resolve_cli_database_path(None, config, workspace_root);
    };
    if fts_db_path.is_absolute() {
        return Ok(fts_db_path.clone());
    }

    Ok(workspace_root.join(fts_db_path))
}

pub fn resolve_cli_fts_analysis_workers(
    config: &Option<PathBuf>,
    workspace_root: &Path,
    analysis_workers: Option<usize>,
    fts_config: &FtsConfig,
) -> ExtractResult<usize> {
    if let Some(analysis_workers) = analysis_workers {
        return Ok(analysis_workers);
    }
    if let Some(analysis_workers) = fts_config.analysis_workers() {
        return Ok(analysis_workers);
    }

    let config_path = resolve_cli_config_path(config, workspace_root)?;
    let Some(config_path) = config_path else {
        return Ok(1);
    };

    let config = load_config(config_path).map_err(ExtractError::config)?;
    Ok(config.extractor().analysis_workers().unwrap_or(1))
}

pub fn resolve_rust_file_mode(
    calls: bool,
    references: bool,
    symbols: bool,
) -> ExtractResult<RustFileMode> {
    let selected = [calls, references, symbols]
        .into_iter()
        .filter(|selected| *selected)
        .count();
    if selected > 1 {
        return Err(ExtractError::response_shape(
            "rust-analyzer",
            "rust-file",
            "--calls, --references, and --symbols are mutually exclusive",
        ));
    }

    if calls {
        Ok(RustFileMode::Calls)
    } else if references {
        Ok(RustFileMode::References)
    } else if symbols {
        Ok(RustFileMode::Symbols)
    } else {
        Ok(RustFileMode::Full)
    }
}

fn resolve_cli_config_path(
    config: &Option<PathBuf>,
    workspace_root: &Path,
) -> ExtractResult<Option<PathBuf>> {
    match config {
        Some(path) => Ok(Some(path.clone())),
        None => discover_config(workspace_root).map_err(ExtractError::config),
    }
}

pub fn resolve_rust_workspace_routes(
    calls: bool,
    references: bool,
    symbols: bool,
) -> WorkspaceExtractionRoutes {
    WorkspaceExtractionRoutes::from_selectors(symbols, references, calls)
}

pub fn resolve_csharp_file_mode(
    calls: bool,
    references: bool,
    symbols: bool,
) -> ExtractResult<CSharpFileMode> {
    let selected = [calls, references, symbols]
        .into_iter()
        .filter(|selected| *selected)
        .count();
    if selected > 1 {
        return Err(ExtractError::response_shape(
            "csharp-language-server",
            "csharp-file",
            "--calls, --references, and --symbols are mutually exclusive",
        ));
    }

    if calls {
        Ok(CSharpFileMode::Calls)
    } else if references {
        Ok(CSharpFileMode::References)
    } else if symbols {
        Ok(CSharpFileMode::Symbols)
    } else {
        Ok(CSharpFileMode::Full)
    }
}

pub fn resolve_csharp_workspace_routes(
    calls: bool,
    references: bool,
    symbols: bool,
) -> WorkspaceExtractionRoutes {
    WorkspaceExtractionRoutes::from_selectors(symbols, references, calls)
}

pub fn resolve_soul_file_mode(references: bool, symbols: bool) -> ExtractResult<SoulFileMode> {
    if references && symbols {
        return Err(ExtractError::response_shape(
            "soul-lsp",
            "soul-file",
            "--references and --symbols are mutually exclusive",
        ));
    }

    if references {
        Ok(SoulFileMode::References)
    } else if symbols {
        Ok(SoulFileMode::Symbols)
    } else {
        Ok(SoulFileMode::Full)
    }
}

pub fn resolve_soul_workspace_routes(references: bool, symbols: bool) -> WorkspaceExtractionRoutes {
    if references || symbols {
        WorkspaceExtractionRoutes::from_selectors(symbols, references, false)
    } else {
        WorkspaceExtractionRoutes::from_selectors(true, true, false)
    }
}

pub fn resolve_soul_lsp_config(
    config: &Option<PathBuf>,
    discovery_start_dir: &Path,
) -> ExtractResult<soul_lsp_lib::SoulLspConfig> {
    let soul_config = resolve_cli_soul_config(config, discovery_start_dir)?;
    Ok(soul_lsp_config_from(&soul_config))
}

pub fn resolve_solution(
    cli_solution: Option<PathBuf>,
    csharp_config: &CSharpConfig,
) -> ExtractResult<PathBuf> {
    let current_dir = env::current_dir()
        .map_err(|source| ExtractError::io("read current directory", None, source))?;
    resolve_solution_from(
        cli_solution,
        csharp_config.solution().cloned(),
        &current_dir,
    )
}

pub fn resolve_solution_from(
    cli_solution: Option<PathBuf>,
    config_solution: Option<PathBuf>,
    current_dir: &Path,
) -> ExtractResult<PathBuf> {
    if let Some(solution) = cli_solution {
        return validate_solution_path(solution, current_dir, "--solution");
    }

    if let Some(solution) = config_solution {
        return validate_solution_path(solution, current_dir, "[csharp].solution");
    }

    discover_solution_in_current_dir(current_dir).ok_or_else(|| {
        ExtractError::response_shape(
            "csharp-language-server",
            "resolve_solution",
            "no C# solution found; pass --solution, set [csharp].solution, or run from a directory containing a .slnx or .sln",
        )
    })
}

pub fn resolve_csharp_extractor_plan(
    config: &Option<PathBuf>,
    discovery_start_dir: &Path,
    cli_binary: Option<PathBuf>,
    cli_solution: Option<PathBuf>,
    process_workers: Option<usize>,
) -> ExtractResult<ResolvedCSharpExtractorPlan> {
    let (csharp_config, config_dir) = resolve_cli_csharp_config(config, discovery_start_dir)?;
    let binary = cli_binary.unwrap_or_else(|| csharp_config.binary().clone());
    let config_solution = csharp_config
        .solution()
        .map(|solution| resolve_config_relative_path(solution, config_dir.as_deref()));
    let solution = resolve_solution_from(cli_solution, config_solution, discovery_start_dir)?;
    let process_workers = process_workers.unwrap_or_else(|| csharp_config.analysis_workers());
    if process_workers == 0 {
        return Err(ExtractError::response_shape(
            "csharp-language-server",
            "csharp extractor plan",
            "--process-workers must be greater than zero",
        ));
    }

    Ok(ResolvedCSharpExtractorPlan::new(
        binary,
        solution,
        csharp_config.log_level().to_string(),
        csharp_config.features().to_vec(),
        process_workers,
        csharp_config.startup_timeout_ms(),
        csharp_config.request_timeout_ms(),
    ))
}

pub fn validate_deleted_rust_file_request(
    workspace_root: PathBuf,
    file_path: &Path,
) -> ExtractResult<(PathBuf, String, String)> {
    let workspace_root = workspace_root
        .canonicalize()
        .map_err(|source| ExtractError::io("canonicalize workspace root", None, source))?;
    let file_path = resolve_deleted_file_path(&workspace_root, file_path)?;

    if file_path.exists() && !file_path.is_file() {
        return Err(ExtractError::invalid_path(
            &file_path,
            &workspace_root,
            "deleted rust-file path must be a file path",
        ));
    }

    let relative_path = workspace_relative_path(&workspace_root, &file_path)?;
    let file_uri = file_uri(&file_path)?;

    Ok((workspace_root, file_uri, relative_path))
}

pub fn resolve_deleted_file_path(
    workspace_root: &Path,
    file_path: &Path,
) -> ExtractResult<PathBuf> {
    let file_path = if file_path.is_absolute() {
        file_path.to_path_buf()
    } else {
        workspace_root.join(file_path)
    };
    let file_path = normalize_lexical_path(&file_path);

    if !file_path.starts_with(workspace_root) {
        return Err(ExtractError::invalid_path(
            &file_path,
            workspace_root,
            "deleted rust-file path is outside the workspace root",
        ));
    }

    Ok(file_path)
}

pub fn normalize_lexical_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(value) => normalized.push(value),
        }
    }

    normalized
}

pub fn symbol_key_belongs_to_file(symbol_key: &str, file_scope_key: &str) -> bool {
    let file_node_key = format!("file:{file_scope_key}");
    symbol_key == file_node_key
        || symbol_key
            .strip_prefix(file_scope_key)
            .is_some_and(|suffix| suffix.starts_with('#'))
}

fn resolve_cli_csharp_config(
    config: &Option<PathBuf>,
    discovery_start_dir: &Path,
) -> ExtractResult<(CSharpConfig, Option<PathBuf>)> {
    let config_path = match config {
        Some(path) => path.clone(),
        None => match discover_config(discovery_start_dir).map_err(ExtractError::config)? {
            Some(path) => path,
            None => discovery_start_dir.join(".refactor-radar/config.toml"),
        },
    };

    ensure_config_with_csharp_defaults(&config_path).map_err(ExtractError::config)?;
    let loaded = load_config(&config_path).map_err(ExtractError::config)?;
    let config_dir = config_path.parent().map(Path::to_path_buf);

    Ok((loaded.csharp().clone(), config_dir))
}

fn resolve_cli_soul_config(
    config: &Option<PathBuf>,
    discovery_start_dir: &Path,
) -> ExtractResult<SoulConfig> {
    let config_path = match config {
        Some(path) => path.clone(),
        None => match discover_config(discovery_start_dir).map_err(ExtractError::config)? {
            Some(path) => path,
            None => discovery_start_dir.join(".refactor-radar/config.toml"),
        },
    };

    ensure_config_with_csharp_defaults(&config_path).map_err(ExtractError::config)?;
    let loaded = load_config(&config_path).map_err(ExtractError::config)?;

    Ok(loaded.soul().clone())
}

fn soul_lsp_config_from(config: &SoulConfig) -> soul_lsp_lib::SoulLspConfig {
    soul_lsp_lib::SoulLspConfig::new(
        soul_lsp_lib::SoulLspScanConfig::new(
            config.scan().excluded_dirs().to_vec(),
            config.scan().excluded_dir_suffixes().to_vec(),
            config.scan().excluded_bin_except_under().to_vec(),
        ),
        config
            .plugins()
            .iter()
            .map(|plugin| {
                soul_lsp_lib::SoulLspPluginConfig::new(
                    plugin.language().to_string(),
                    plugin.path().clone(),
                )
            })
            .collect(),
    )
}

fn resolve_config_relative_path(path: &Path, config_dir: Option<&Path>) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }

    if let Some(config_dir) = config_dir {
        let config_relative = config_dir.join(path);
        if config_relative.is_file() {
            return config_relative;
        }
    }

    path.to_path_buf()
}

fn validate_solution_path(
    solution: PathBuf,
    current_dir: &Path,
    source: &'static str,
) -> ExtractResult<PathBuf> {
    let resolved = if solution.is_absolute() {
        solution
    } else {
        current_dir.join(solution)
    };
    if !resolved.is_file() {
        return Err(ExtractError::invalid_path(
            resolved,
            current_dir,
            format!("{source} must point to an existing .slnx or .sln file"),
        ));
    }
    if !is_solution_file(&resolved) {
        return Err(ExtractError::invalid_path(
            resolved,
            current_dir,
            format!("{source} must point to a .slnx or .sln file"),
        ));
    }

    Ok(normalize_lexical_path(&resolved))
}

fn discover_solution_in_current_dir(current_dir: &Path) -> Option<PathBuf> {
    let mut candidates = fs::read_dir(current_dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && is_solution_file(path))
        .collect::<Vec<_>>();
    candidates.sort();

    candidates
        .iter()
        .find(|path| extension_is(path, "slnx"))
        .or_else(|| candidates.iter().find(|path| extension_is(path, "sln")))
        .map(|path| normalize_lexical_path(path))
}

fn is_solution_file(path: &Path) -> bool {
    extension_is(path, "slnx") || extension_is(path, "sln")
}

fn extension_is(path: &Path, expected: &str) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(expected))
}
