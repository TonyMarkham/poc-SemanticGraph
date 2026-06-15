mod config;
mod database_config;
mod error;
mod load_options;
mod raw_config;
mod raw_database_config;
mod raw_writer_config;
mod resolved_database_path;
mod resolved_database_path_source;
#[cfg(test)]
mod tests;
mod writer_config;

// ---------------------------------------------------------------------------------------------- //

pub(crate) use raw_database_config::RawDatabaseConfig;
pub(crate) use raw_writer_config::RawWriterConfig;

pub use config::Config;
pub use database_config::DatabaseConfig;
pub use error::{ConfigError, ConfigResult};
pub use load_options::LoadOptions;
pub use raw_config::load_config;
pub use resolved_database_path::ResolvedDatabasePath;
pub use resolved_database_path_source::ResolvedDatabasePathSource;
pub use writer_config::WriterConfig;

// ---------------------------------------------------------------------------------------------- //

use std::{
    env,
    path::{Path, PathBuf},
};

const CONFIG_RELATIVE_PATH: &str = ".refactor-radar/config.toml";

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

fn absolute_start_dir(start_dir: &Path) -> ConfigResult<PathBuf> {
    if start_dir.is_absolute() {
        return Ok(start_dir.to_path_buf());
    }

    let current_dir = env::current_dir()
        .map_err(|source| ConfigError::io("read current directory", None, source))?;
    Ok(current_dir.join(start_dir))
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
