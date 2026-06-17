use crate::{
    ExtractError, ExtractResult,
    fts::{FtsExtractionOptions, FtsFileLanguage, FtsSkipReason, normalize_relative_path},
};

use semantic_graph_config::FtsConfig;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FtsExclusionSet {
    config_directories: BTreeSet<String>,
    config_files: BTreeSet<String>,
    submodule_directories: BTreeSet<String>,
    options: FtsExtractionOptions,
}

impl FtsExclusionSet {
    pub fn new(
        workspace_root: &Path,
        config: &FtsConfig,
        options: FtsExtractionOptions,
    ) -> ExtractResult<Self> {
        let submodule_directories = if options.no_submodules() {
            discover_submodule_paths(workspace_root)?
        } else {
            BTreeSet::new()
        };

        Ok(Self {
            config_directories: config.ignore_directories().iter().cloned().collect(),
            config_files: config.ignore_files().iter().cloned().collect(),
            submodule_directories,
            options,
        })
    }

    pub fn skip_directory_reason(&self, relative_path: &str) -> Option<FtsSkipReason> {
        if relative_path.is_empty() {
            return None;
        }
        if self
            .config_directories
            .iter()
            .any(|directory| path_is_equal_or_descendant(relative_path, directory))
        {
            return Some(FtsSkipReason::Config);
        }
        if self.options.no_submodules()
            && self
                .submodule_directories
                .iter()
                .any(|directory| path_is_equal_or_descendant(relative_path, directory))
        {
            return Some(FtsSkipReason::NoSubmodules);
        }

        None
    }

    pub fn skip_file_reason(
        &self,
        relative_path: &str,
        language: FtsFileLanguage,
    ) -> Option<FtsSkipReason> {
        if self
            .config_directories
            .iter()
            .any(|directory| path_is_equal_or_descendant(relative_path, directory))
            || self.config_files.contains(relative_path)
        {
            return Some(FtsSkipReason::Config);
        }
        if self.options.no_submodules()
            && self
                .submodule_directories
                .iter()
                .any(|directory| path_is_equal_or_descendant(relative_path, directory))
        {
            return Some(FtsSkipReason::NoSubmodules);
        }
        if self.options.no_rust() && language == FtsFileLanguage::Rust {
            return Some(FtsSkipReason::NoRust);
        }
        if self.options.no_csharp() && language == FtsFileLanguage::CSharp {
            return Some(FtsSkipReason::NoCSharp);
        }

        None
    }
}

fn path_is_equal_or_descendant(path: &str, directory: &str) -> bool {
    path == directory
        || path
            .strip_prefix(directory)
            .is_some_and(|tail| tail.starts_with('/'))
}

fn discover_submodule_paths(workspace_root: &Path) -> ExtractResult<BTreeSet<String>> {
    let mut paths = BTreeSet::new();
    collect_submodule_paths(workspace_root, "", &mut paths)?;
    Ok(paths)
}

fn collect_submodule_paths(
    directory: &Path,
    relative_prefix: &str,
    paths: &mut BTreeSet<String>,
) -> ExtractResult<()> {
    let gitmodules = directory.join(".gitmodules");
    if !gitmodules.is_file() {
        return Ok(());
    }

    let contents = fs::read_to_string(&gitmodules).map_err(|source| {
        ExtractError::io(
            "read .gitmodules for FTS submodule exclusions",
            Some(gitmodules.clone()),
            source,
        )
    })?;
    for path in parse_gitmodule_paths(&contents) {
        let relative_path = if relative_prefix.is_empty() {
            normalize_relative_path(Path::new(&path))
        } else {
            normalize_relative_path(&PathBuf::from(relative_prefix).join(&path))
        };
        paths.insert(relative_path.clone());
        collect_submodule_paths(&directory.join(path), &relative_path, paths)?;
    }

    Ok(())
}

fn parse_gitmodule_paths(contents: &str) -> Vec<String> {
    contents
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let (key, value) = trimmed.split_once('=')?;
            (key.trim() == "path").then(|| value.trim().to_string())
        })
        .filter(|value| !value.is_empty())
        .collect()
}
