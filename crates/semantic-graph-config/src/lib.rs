mod config;
mod csharp_config;
mod database_config;
mod error;
mod extractor_config;
mod extractor_mode;
mod load_options;
mod raw_config;
mod raw_csharp_config;
mod raw_database_config;
mod raw_extractor_config;
mod raw_writer_config;
mod resolved_database_path;
mod resolved_database_path_source;
#[cfg(test)]
mod tests;
mod writer_config;

// ---------------------------------------------------------------------------------------------- //

pub(crate) use raw_csharp_config::RawCSharpConfig;
pub(crate) use raw_database_config::RawDatabaseConfig;
pub(crate) use raw_extractor_config::RawExtractorConfig;
pub(crate) use raw_writer_config::RawWriterConfig;

pub use config::Config;
pub use csharp_config::CSharpConfig;
pub use database_config::DatabaseConfig;
pub use error::{ConfigError, ConfigResult};
pub use extractor_config::ExtractorConfig;
pub use extractor_mode::ExtractorMode;
pub use load_options::LoadOptions;
pub use raw_config::load_config;
pub use resolved_database_path::ResolvedDatabasePath;
pub use resolved_database_path_source::ResolvedDatabasePathSource;
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

[csharp]
binary = "csharp-ls"
solution = "SemanticGraph.Visualizer.slnx"
log_level = "warning"
features = []
analysis_workers = 1
startup_timeout_ms = 120000
request_timeout_ms = 30000
"#;
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
        fs::write(config_path, DEFAULT_CONFIG_TEMPLATE).map_err(|source| {
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
    let parsed = toml::from_str::<toml::Value>(&contents)
        .map_err(|source| ConfigError::toml(config_path, source))?;
    let root = parsed.as_table().ok_or_else(|| {
        ConfigError::invalid_csharp_setting("csharp", "config root must be a table")
    })?;
    let missing_lines = match root.get("csharp") {
        Some(value) => {
            let table = value.as_table().ok_or_else(|| {
                ConfigError::invalid_csharp_setting("csharp", "must be a TOML table")
            })?;
            CSHARP_DEFAULT_LINES
                .iter()
                .filter_map(|(key, line)| (!table.contains_key(*key)).then_some(*line))
                .collect::<Vec<_>>()
        }
        None => CSHARP_DEFAULT_LINES
            .iter()
            .map(|(_key, line)| *line)
            .collect::<Vec<_>>(),
    };

    if missing_lines.is_empty() {
        return Ok(());
    }

    let updated = if root.contains_key("csharp") {
        insert_missing_csharp_lines(&contents, &missing_lines)
    } else {
        append_csharp_table(&contents)
    };
    fs::write(config_path, updated).map_err(|source| {
        ConfigError::io(
            "update refactor radar config with csharp defaults",
            Some(config_path.to_path_buf()),
            source,
        )
    })
}

fn absolute_start_dir(start_dir: &Path) -> ConfigResult<PathBuf> {
    if start_dir.is_absolute() {
        return Ok(start_dir.to_path_buf());
    }

    let current_dir = env::current_dir()
        .map_err(|source| ConfigError::io("read current directory", None, source))?;
    Ok(current_dir.join(start_dir))
}

fn append_csharp_table(contents: &str) -> String {
    let mut updated = contents.to_string();
    if !updated.ends_with('\n') {
        updated.push('\n');
    }
    if !updated.ends_with("\n\n") {
        updated.push('\n');
    }
    updated.push_str(CSHARP_TABLE_HEADER);
    updated.push('\n');
    for (_key, line) in CSHARP_DEFAULT_LINES {
        updated.push_str(line);
        updated.push('\n');
    }
    updated
}

fn insert_missing_csharp_lines(contents: &str, missing_lines: &[&str]) -> String {
    let lines = contents.lines().collect::<Vec<_>>();
    let Some(section_start) = lines
        .iter()
        .position(|line| line.trim() == CSHARP_TABLE_HEADER)
    else {
        return append_csharp_table(contents);
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
