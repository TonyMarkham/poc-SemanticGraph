use crate::{
    ExtractError, ExtractResult,
    fts::{
        FtsDiscoveredFile, FtsDiscoveryResult, FtsExclusionSet, FtsFileLanguage, FtsSkipReason,
        normalize_relative_path,
    },
};

use std::{fs, path::Path};

pub struct FtsFileDiscovery;

impl FtsFileDiscovery {
    pub fn discover(
        workspace_root: &Path,
        exclusions: &FtsExclusionSet,
    ) -> ExtractResult<FtsDiscoveryResult> {
        let mut result = FtsDiscoveryResult::default();
        walk_directory(workspace_root, workspace_root, exclusions, &mut result)?;
        Ok(result)
    }
}

fn walk_directory(
    workspace_root: &Path,
    directory: &Path,
    exclusions: &FtsExclusionSet,
    result: &mut FtsDiscoveryResult,
) -> ExtractResult<()> {
    let mut entries = fs::read_dir(directory)
        .map_err(|source| {
            ExtractError::io("read FTS directory", Some(directory.to_path_buf()), source)
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| {
            ExtractError::io(
                "read FTS directory entry",
                Some(directory.to_path_buf()),
                source,
            )
        })?;
    entries.sort_by_key(|entry| relative_path(workspace_root, &entry.path()));

    for entry in entries {
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|source| ExtractError::io("read FTS file type", Some(path.clone()), source))?;
        let relative_path = relative_path(workspace_root, &path);

        if file_type.is_dir() {
            if let Some(reason) = exclusions.skip_directory_reason(&relative_path) {
                result.count_skipped_directory();
                count_skip_reason(result, reason);
                continue;
            }
            walk_directory(workspace_root, &path, exclusions, result)?;
        } else if file_type.is_file() {
            result.count_scanned_file();
            let language = FtsFileLanguage::from_path(&path);
            if let Some(reason) = exclusions.skip_file_reason(&relative_path, language) {
                result.count_skipped_file();
                count_skip_reason(result, reason);
                continue;
            }
            result.push_file(FtsDiscoveredFile::new(path, relative_path, language));
        }
    }

    Ok(())
}

fn relative_path(workspace_root: &Path, path: &Path) -> String {
    path.strip_prefix(workspace_root)
        .map(normalize_relative_path)
        .unwrap_or_else(|_| normalize_relative_path(path))
}

fn count_skip_reason(result: &mut FtsDiscoveryResult, reason: FtsSkipReason) {
    match reason {
        FtsSkipReason::Config => result.count_skipped_by_config(),
        FtsSkipReason::NoRust => result.count_skipped_by_no_rust(),
        FtsSkipReason::NoCSharp => result.count_skipped_by_no_csharp(),
        FtsSkipReason::NoSubmodules => result.count_skipped_by_no_submodules(),
        FtsSkipReason::BinaryOrUnreadable => {}
    }
}
