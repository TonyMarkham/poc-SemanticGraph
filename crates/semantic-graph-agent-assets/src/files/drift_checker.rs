use crate::{
    constants::manifest::HOST_CODEX,
    error::{AgentAssetsError, AgentAssetsResult},
    files::{
        asset_writer::AssetWriter, drift_check_report::DriftCheckReport, drift_report::DriftReport,
        expected_artifact_files::collect_files, temp_render_root::create_temp_render_root,
    },
    manifest::AssetManifest,
    render::RenderedAsset,
};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

pub struct DriftChecker;

impl DriftChecker {
    pub fn check(
        repo_root: &Path,
        manifest: &AssetManifest,
        assets: &[RenderedAsset],
    ) -> AgentAssetsResult<DriftCheckReport> {
        let expected_root = manifest.expected_root(repo_root);
        let temp_root = create_temp_render_root()?;
        let temp_expected_root = temp_root.join(HOST_CODEX);

        let result = Self::check_with_temp_root(&expected_root, &temp_expected_root, assets);
        let cleanup = fs::remove_dir_all(&temp_root).map_err(|source| {
            AgentAssetsError::io("remove temporary render directory", Some(temp_root), source)
        });

        match (result, cleanup) {
            (Ok(report), Ok(())) => Ok(report),
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
        }
    }

    fn check_with_temp_root(
        expected_root: &Path,
        temp_expected_root: &Path,
        assets: &[RenderedAsset],
    ) -> AgentAssetsResult<DriftCheckReport> {
        AssetWriter::write_to_root(temp_expected_root, assets, false)?;

        let mut declared_paths = BTreeSet::new();
        let mut drift = DriftReport::default();
        let mut checked = Vec::new();

        for asset in assets {
            declared_paths.insert(asset.output_path().clone());
            let expected_path = expected_root.join(asset.output_path());
            let rendered_path = temp_expected_root.join(asset.output_path());

            let rendered = fs::read_to_string(&rendered_path).map_err(|source| {
                AgentAssetsError::io(
                    "read rendered temporary artifact",
                    Some(rendered_path),
                    source,
                )
            })?;

            match fs::read_to_string(&expected_path) {
                Ok(existing) if existing == rendered => {
                    checked.push(asset.output_path().clone());
                }
                Ok(_) => drift.changed.push(asset.output_path().clone()),
                Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                    drift.missing.push(asset.output_path().clone());
                }
                Err(source) => {
                    return Err(AgentAssetsError::io(
                        "read expected artifact",
                        Some(expected_path),
                        source,
                    ));
                }
            }
        }

        for existing in collect_files(expected_root)? {
            if !declared_paths.contains(&existing) {
                drift.stale.push(existing);
            }
        }

        sort_paths(&mut checked);
        drift.sort_paths();

        if drift.is_empty() {
            Ok(DriftCheckReport { checked })
        } else {
            Err(AgentAssetsError::drift(drift.message()))
        }
    }
}

fn sort_paths(paths: &mut [PathBuf]) {
    paths.sort_by_key(|path| path.to_string_lossy().to_string());
}
