use crate::{
    error::{AgentAssetsError, AgentAssetsResult},
    files::{asset_write_report::AssetWriteReport, expected_artifact_files::collect_files},
    manifest::AssetManifest,
    render::RenderedAsset,
};
use std::{collections::BTreeSet, fs, path::Path};

pub struct AssetWriter;

impl AssetWriter {
    pub fn write(
        repo_root: &Path,
        manifest: &AssetManifest,
        assets: &[RenderedAsset],
    ) -> AgentAssetsResult<AssetWriteReport> {
        let expected_root = manifest.expected_root(repo_root);
        Self::write_to_root(&expected_root, assets, true)
    }

    pub fn write_to_root(
        expected_root: &Path,
        assets: &[RenderedAsset],
        remove_stale: bool,
    ) -> AgentAssetsResult<AssetWriteReport> {
        fs::create_dir_all(expected_root).map_err(|source| {
            AgentAssetsError::io(
                "create expected artifact root",
                Some(expected_root.to_path_buf()),
                source,
            )
        })?;

        let mut report = AssetWriteReport::default();
        let mut declared_paths = BTreeSet::new();

        for asset in assets {
            declared_paths.insert(asset.output_path().clone());
            let output_path = expected_root.join(asset.output_path());
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent).map_err(|source| {
                    AgentAssetsError::io(
                        "create parent directory",
                        Some(parent.to_path_buf()),
                        source,
                    )
                })?;
            }

            match fs::read_to_string(&output_path) {
                Ok(existing) if existing == asset.content() => {
                    report.unchanged.push(asset.output_path().clone());
                }
                Ok(_) => {
                    fs::write(&output_path, asset.content()).map_err(|source| {
                        AgentAssetsError::io(
                            "write expected artifact",
                            Some(output_path.clone()),
                            source,
                        )
                    })?;
                    report.updated.push(asset.output_path().clone());
                }
                Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                    fs::write(&output_path, asset.content()).map_err(|source| {
                        AgentAssetsError::io(
                            "write expected artifact",
                            Some(output_path.clone()),
                            source,
                        )
                    })?;
                    report.created.push(asset.output_path().clone());
                }
                Err(source) => {
                    return Err(AgentAssetsError::io(
                        "read expected artifact",
                        Some(output_path),
                        source,
                    ));
                }
            }
        }

        if remove_stale {
            for path in collect_files(expected_root)? {
                if !declared_paths.contains(&path) {
                    let output_path = expected_root.join(&path);
                    fs::remove_file(&output_path).map_err(|source| {
                        AgentAssetsError::io(
                            "remove stale expected artifact",
                            Some(output_path),
                            source,
                        )
                    })?;
                    report.removed.push(path);
                }
            }
        }

        report.sort_paths();
        Ok(report)
    }
}
