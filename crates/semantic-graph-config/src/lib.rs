mod config;
mod csharp_config;
mod database_config;
mod error;
mod extractor_config;
mod extractor_mode;
mod fts_config;
mod load_options;
mod query_service_config;
mod query_service_config_values;
mod raw_config;
mod raw_csharp_config;
mod raw_database_config;
mod raw_extractor_config;
mod raw_fts_config;
mod raw_query_service_config;
mod raw_soul_config;
mod raw_soul_plugin_config;
mod raw_soul_scan_config;
mod raw_writer_config;
mod resolved_database_path;
mod resolved_database_path_source;
mod soul_config;
mod soul_plugin_config;
mod soul_scan_config;
#[cfg(test)]
mod tests;
mod writer_config;

// ---------------------------------------------------------------------------------------------- //

pub(crate) use raw_csharp_config::RawCSharpConfig;
pub(crate) use raw_database_config::RawDatabaseConfig;
pub(crate) use raw_extractor_config::RawExtractorConfig;
pub(crate) use raw_fts_config::RawFtsConfig;
pub(crate) use raw_query_service_config::RawQueryServiceConfig;
pub(crate) use raw_soul_config::RawSoulConfig;
pub(crate) use raw_soul_plugin_config::RawSoulPluginConfig;
pub(crate) use raw_soul_scan_config::RawSoulScanConfig;
pub(crate) use raw_writer_config::RawWriterConfig;

pub use config::Config;
pub use csharp_config::CSharpConfig;
pub use database_config::DatabaseConfig;
pub use error::{ConfigError, ConfigResult};
pub use extractor_config::ExtractorConfig;
pub use extractor_mode::ExtractorMode;
pub use fts_config::FtsConfig;
pub use load_options::LoadOptions;
pub use query_service_config::QueryServiceConfig;
pub use query_service_config_values::QueryServiceConfigValues;
pub use raw_config::load_config;
pub use resolved_database_path::ResolvedDatabasePath;
pub use resolved_database_path_source::ResolvedDatabasePathSource;
pub use soul_config::SoulConfig;
pub use soul_plugin_config::SoulPluginConfig;
pub use soul_scan_config::SoulScanConfig;
pub use writer_config::WriterConfig;

// ---------------------------------------------------------------------------------------------- //

use std::{
    env, fs,
    path::{Path, PathBuf},
};

const CONFIG_RELATIVE_PATH: &str = ".refactor-radar/config.toml";
const DEFAULT_CONFIG_TEMPLATE: &str = r#"[database]
path = "content.db"

[writer]
queue_capacity = 4096
max_rows_per_commit = 1000
max_millis_per_commit = 250
busy_timeout_ms = 5000

[query-service]
latest_run_limit = 10
max_search_limit = 50
max_projection_limit = 1000
max_neighbors_limit = 100
max_file_edge_limit = 200
max_route_status_limit = 200
max_shortest_path_depth = 12
max_shortest_path_visited = 5000

[fts]
db_path = ".refactor-radar/fts.db"
analysis_workers = 8
max_indexed_file_bytes = 209715200
ignore-directories = []
ignore-files = []

[csharp]
binary = "csharp-ls"
solution = "SemanticGraph.Visualizer.slnx"
log_level = "warning"
features = []
analysis_workers = 1
startup_timeout_ms = 120000
request_timeout_ms = 30000

[soul.scan]
excluded_dirs = [".git", ".soul", "target", ".idea", ".vscode", ".vs", ".codex", "node_modules", "obj"]
excluded_dir_suffixes = ["Tests", ".Tests", "tests", ".tests"]
excluded_bin_except_under = ["src"]
"#;
const FTS_TABLE_HEADER: &str = "[fts]";
const FTS_DEFAULT_LINES: [(&str, &str); 5] = [
    ("db_path", "db_path = \".refactor-radar/fts.db\""),
    ("analysis_workers", "analysis_workers = 8"),
    (
        "max_indexed_file_bytes",
        "max_indexed_file_bytes = 209715200",
    ),
    ("ignore-directories", "ignore-directories = []"),
    ("ignore-files", "ignore-files = []"),
];
const CSHARP_TABLE_HEADER: &str = "[csharp]";
const CSHARP_DEFAULT_LINES: [(&str, &str); 7] = [
    ("binary", "binary = \"csharp-ls\""),
    ("solution", "solution = \"SemanticGraph.Visualizer.slnx\""),
    ("log_level", "log_level = \"warning\""),
    ("features", "features = []"),
    ("analysis_workers", "analysis_workers = 1"),
    ("startup_timeout_ms", "startup_timeout_ms = 120000"),
    ("request_timeout_ms", "request_timeout_ms = 30000"),
];
const SOUL_SCAN_TABLE_HEADER: &str = "[soul.scan]";
const SOUL_SCAN_DEFAULT_LINES: [(&str, &str); 3] = [
    (
        "excluded_dirs",
        "excluded_dirs = [\".git\", \".soul\", \"target\", \".idea\", \".vscode\", \".vs\", \".codex\", \"node_modules\", \"obj\"]",
    ),
    (
        "excluded_dir_suffixes",
        "excluded_dir_suffixes = [\"Tests\", \".Tests\", \"tests\", \".tests\"]",
    ),
    (
        "excluded_bin_except_under",
        "excluded_bin_except_under = [\"src\"]",
    ),
];
const SOUL_DEFAULT_PLUGIN_HEADER: &str = "[[soul.plugins]]";

pub fn discover_config(start_dir: impl AsRef<Path>) -> ConfigResult<Option<PathBuf>> {
    let start_dir = absolute_start_dir(start_dir.as_ref())?;

    for ancestor in start_dir.ancestors() {
        let candidate = ancestor.join(CONFIG_RELATIVE_PATH);
        if candidate.is_file() {
            return Ok(Some(candidate));
        }
    }

    Ok(None)
}

pub fn ensure_config_with_csharp_defaults(config_path: impl AsRef<Path>) -> ConfigResult<()> {
    let config_path = config_path.as_ref();
    if !config_path.exists() {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).map_err(|source| {
                ConfigError::io(
                    "create refactor radar config directory",
                    Some(parent.to_path_buf()),
                    source,
                )
            })?;
        }
        fs::write(config_path, default_config_template()).map_err(|source| {
            ConfigError::io(
                "write default refactor radar config",
                Some(config_path.to_path_buf()),
                source,
            )
        })?;
        return Ok(());
    }

    let contents = fs::read_to_string(config_path).map_err(|source| {
        ConfigError::io(
            "read refactor radar config",
            Some(config_path.to_path_buf()),
            source,
        )
    })?;
    let updated =
        ensure_default_table_lines(&contents, config_path, FTS_TABLE_HEADER, &FTS_DEFAULT_LINES)?;
    let updated = ensure_default_table_lines(
        &updated,
        config_path,
        CSHARP_TABLE_HEADER,
        &CSHARP_DEFAULT_LINES,
    )?;
    let updated = ensure_default_table_lines(
        &updated,
        config_path,
        SOUL_SCAN_TABLE_HEADER,
        &SOUL_SCAN_DEFAULT_LINES,
    )?;
    let updated = ensure_default_soul_plugins(&updated, config_path)?;

    if updated == contents {
        return Ok(());
    }
    fs::write(config_path, updated).map_err(|source| {
        ConfigError::io(
            "update refactor radar config defaults",
            Some(config_path.to_path_buf()),
            source,
        )
    })
}

fn default_config_template() -> String {
    let mut template = DEFAULT_CONFIG_TEMPLATE.to_string();
    template.push_str(&default_soul_plugins_section());
    template
}

fn absolute_start_dir(start_dir: &Path) -> ConfigResult<PathBuf> {
    if start_dir.is_absolute() {
        return Ok(start_dir.to_path_buf());
    }

    let current_dir = env::current_dir()
        .map_err(|source| ConfigError::io("read current directory", None, source))?;
    Ok(current_dir.join(start_dir))
}

fn ensure_default_soul_plugins(contents: &str, config_path: &Path) -> ConfigResult<String> {
    let parsed = toml::from_str::<toml::Value>(contents)
        .map_err(|source| ConfigError::toml(config_path, source))?;
    let root = parsed
        .as_table()
        .ok_or_else(|| ConfigError::invalid_soul_setting("soul", "config root must be a table"))?;
    let plugins = root
        .get("soul")
        .and_then(toml::Value::as_table)
        .and_then(|soul| soul.get("plugins"));

    match plugins {
        Some(value) if value.as_array().is_some() => Ok(contents.to_string()),
        Some(_) => Err(ConfigError::invalid_soul_setting(
            "soul.plugins",
            "must be an array of plugin tables",
        )),
        None => {
            let mut updated = contents.to_string();
            if !updated.ends_with('\n') {
                updated.push('\n');
            }
            if !updated.ends_with("\n\n") {
                updated.push('\n');
            }
            updated.push_str(&default_soul_plugins_section());
            Ok(updated)
        }
    }
}

fn default_soul_plugins_section() -> String {
    let suffix = std::env::consts::DLL_SUFFIX;
    format!(
        r#"{SOUL_DEFAULT_PLUGIN_HEADER}
language = "rust"
path = "./.soul/plugins/rust{suffix}"

{SOUL_DEFAULT_PLUGIN_HEADER}
language = "csharp"
path = "./.soul/plugins/csharp{suffix}"
"#
    )
}

fn ensure_default_table_lines(
    contents: &str,
    config_path: &Path,
    table_header: &str,
    default_lines: &[(&str, &str)],
) -> ConfigResult<String> {
    let table_name = table_header.trim_matches(['[', ']']);
    let parsed = toml::from_str::<toml::Value>(contents)
        .map_err(|source| ConfigError::toml(config_path, source))?;
    let root = parsed
        .as_table()
        .ok_or_else(|| invalid_default_table_setting(table_name, "config root must be a table"))?;
    let table = table_at_path(root, table_name)?;
    let missing_lines = match table {
        Some(table) => default_lines
            .iter()
            .filter_map(|(key, line)| (!table.contains_key(*key)).then_some(*line))
            .collect::<Vec<_>>(),
        None => default_lines
            .iter()
            .map(|(_key, line)| *line)
            .collect::<Vec<_>>(),
    };

    if missing_lines.is_empty() {
        return Ok(contents.to_string());
    }

    if table.is_some() {
        Ok(insert_missing_table_lines(
            contents,
            table_header,
            &missing_lines,
        ))
    } else {
        Ok(append_table(contents, table_header, default_lines))
    }
}

fn table_at_path<'a>(
    root: &'a toml::value::Table,
    table_name: &str,
) -> ConfigResult<Option<&'a toml::value::Table>> {
    let mut current = root;
    for part in table_name.split('.') {
        let Some(value) = current.get(part) else {
            return Ok(None);
        };
        let Some(table) = value.as_table() else {
            return Err(invalid_default_table_setting(
                table_name,
                "must be a TOML table",
            ));
        };
        current = table;
    }

    Ok(Some(current))
}

fn invalid_default_table_setting(table_name: &str, message: &str) -> ConfigError {
    match table_name.split('.').next().unwrap_or(table_name) {
        "fts" => ConfigError::invalid_fts_setting(table_name, message),
        "csharp" => ConfigError::invalid_csharp_setting(table_name, message),
        "soul" => ConfigError::invalid_soul_setting(table_name, message),
        _ => ConfigError::invalid_csharp_setting(table_name, message),
    }
}

fn append_table(contents: &str, table_header: &str, default_lines: &[(&str, &str)]) -> String {
    let mut updated = contents.to_string();
    if !updated.ends_with('\n') {
        updated.push('\n');
    }
    if !updated.ends_with("\n\n") {
        updated.push('\n');
    }
    updated.push_str(table_header);
    updated.push('\n');
    for (_key, line) in default_lines {
        updated.push_str(line);
        updated.push('\n');
    }
    updated
}

fn insert_missing_table_lines(
    contents: &str,
    table_header: &str,
    missing_lines: &[&str],
) -> String {
    let lines = contents.lines().collect::<Vec<_>>();
    let Some(section_start) = lines.iter().position(|line| line.trim() == table_header) else {
        return append_lines_table(contents, table_header, missing_lines);
    };
    let section_end = lines
        .iter()
        .enumerate()
        .skip(section_start + 1)
        .find_map(|(index, line)| is_table_header(line).then_some(index))
        .unwrap_or(lines.len());
    let mut updated = String::new();

    for line in &lines[..section_end] {
        updated.push_str(line);
        updated.push('\n');
    }
    for line in missing_lines {
        updated.push_str(line);
        updated.push('\n');
    }
    for line in &lines[section_end..] {
        updated.push_str(line);
        updated.push('\n');
    }

    updated
}

fn append_lines_table(contents: &str, table_header: &str, missing_lines: &[&str]) -> String {
    let mut updated = contents.to_string();
    if !updated.ends_with('\n') {
        updated.push('\n');
    }
    if !updated.ends_with("\n\n") {
        updated.push('\n');
    }
    updated.push_str(table_header);
    updated.push('\n');
    for line in missing_lines {
        updated.push_str(line);
        updated.push('\n');
    }
    updated
}

fn is_table_header(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('[') && trimmed.ends_with(']')
}

pub fn resolve_database_path(options: LoadOptions) -> ConfigResult<ResolvedDatabasePath> {
    if let Some(path) = options.explicit_database_path {
        return Ok(ResolvedDatabasePath::new(
            path,
            ResolvedDatabasePathSource::ExplicitDatabasePath,
        ));
    }

    if let Some(config_path) = options.explicit_config_path {
        let database_path = database_path_from_config(&config_path)?;
        return Ok(ResolvedDatabasePath::new(
            database_path,
            ResolvedDatabasePathSource::ExplicitConfig,
        ));
    }

    let start_dir = match options.discovery_start_dir {
        Some(path) => path,
        None => env::current_dir()
            .map_err(|source| ConfigError::io("read current directory", None, source))?,
    };

    if let Some(config_path) = discover_config(&start_dir)? {
        let database_path = database_path_from_config(&config_path)?;
        return Ok(ResolvedDatabasePath::new(
            database_path,
            ResolvedDatabasePathSource::DiscoveredConfig,
        ));
    }

    if let Some(path) = options.default_database_path {
        return Ok(ResolvedDatabasePath::new(
            path,
            ResolvedDatabasePathSource::Default,
        ));
    }

    Err(ConfigError::missing_database_path(None))
}

fn database_path_from_config(config_path: &Path) -> ConfigResult<PathBuf> {
    let config = load_config(config_path)?;
    let database_path = config.database().path();
    if database_path.is_absolute() {
        return Ok(database_path.clone());
    }

    Ok(config_directory(config_path)?.join(database_path))
}

fn config_directory(config_path: &Path) -> ConfigResult<PathBuf> {
    let absolute_config_path = if config_path.is_absolute() {
        config_path.to_path_buf()
    } else {
        env::current_dir()
            .map_err(|source| ConfigError::io("read current directory", None, source))?
            .join(config_path)
    };

    Ok(absolute_config_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(".")))
}
