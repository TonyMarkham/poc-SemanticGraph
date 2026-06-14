use crate::{
    RustAnalyzerLibError, RustAnalyzerLibResult,
    model::{RustPackage, RustSourceFile, RustTarget, RustTargetKind, RustWorkspaceModel},
};

use paths::AbsPathBuf;
use project_model::{CargoConfig, ProjectManifest, ProjectWorkspace, ProjectWorkspaceKind};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};
use syntax::{AstNode, Edition, SourceFile, ast, ast::HasName};

pub fn load_workspace(
    workspace_root: impl AsRef<Path>,
) -> RustAnalyzerLibResult<RustWorkspaceModel> {
    let workspace_root = absolute_path(workspace_root.as_ref(), "canonicalize workspace root")?;
    let abs_root = AbsPathBuf::assert_utf8(workspace_root.clone());
    let manifest = ProjectManifest::discover_single(&abs_root).map_err(|source| {
        RustAnalyzerLibError::project("discover Rust project manifest", source)
    })?;
    let cargo_config = CargoConfig {
        set_test: true,
        ..CargoConfig::default()
    };
    let project_workspace =
        ProjectWorkspace::load(manifest, &cargo_config, &|_| {}).map_err(|source| {
            RustAnalyzerLibError::project("load rust-analyzer project workspace", source)
        })?;
    let packages = packages_from_workspace(&project_workspace);
    let member_package_roots = packages
        .iter()
        .filter(|package| package.is_workspace_member)
        .map(|package| package.package_root.clone())
        .collect::<Vec<_>>();

    let mut source_files = discover_module_source_files(&packages)?
        .into_iter()
        .filter(|path| {
            member_package_roots
                .iter()
                .any(|package_root| path.starts_with(package_root))
        })
        .map(|path| RustSourceFile { path })
        .collect::<Vec<_>>();
    source_files.sort_by(|left, right| left.path.cmp(&right.path));
    source_files.dedup_by(|left, right| left.path == right.path);

    Ok(RustWorkspaceModel {
        workspace_root,
        packages,
        source_files,
    })
}

fn discover_module_source_files(packages: &[RustPackage]) -> RustAnalyzerLibResult<Vec<PathBuf>> {
    let mut visited = HashSet::new();
    let mut stack = packages
        .iter()
        .filter(|package| package.is_workspace_member)
        .flat_map(|package| package.targets.iter())
        .map(|target| target.root_file.clone())
        .collect::<Vec<_>>();

    while let Some(path) = stack.pop() {
        let path = path.canonicalize().map_err(|source| {
            RustAnalyzerLibError::io("canonicalize Rust module", Some(path.clone()), source)
        })?;
        if !visited.insert(path.clone()) {
            continue;
        }

        for child in external_module_files(&path)? {
            stack.push(child);
        }
    }

    Ok(visited.into_iter().collect())
}

fn external_module_files(file_path: &Path) -> RustAnalyzerLibResult<Vec<PathBuf>> {
    let source = fs::read_to_string(file_path).map_err(|source| {
        RustAnalyzerLibError::io(
            "read Rust module source",
            Some(file_path.to_path_buf()),
            source,
        )
    })?;
    let parse = SourceFile::parse(&source, Edition::CURRENT);
    let file = parse.tree();
    let mut module_files = Vec::new();

    for module in file.syntax().descendants().filter_map(ast::Module::cast) {
        if module.item_list().is_some() || module.semicolon_token().is_none() {
            continue;
        }

        let Some(name) = module.name() else {
            continue;
        };
        let module_name = name.text().to_string();
        module_files.extend(resolve_module_candidates(file_path, &module_name));
    }

    Ok(module_files)
}

fn resolve_module_candidates(current_file: &Path, module_name: &str) -> Vec<PathBuf> {
    let Some(parent) = current_file.parent() else {
        return Vec::new();
    };

    let stem = current_file
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let module_dir = if matches!(stem, "lib" | "main" | "mod") {
        parent.to_path_buf()
    } else {
        parent.join(stem)
    };

    [
        module_dir.join(format!("{module_name}.rs")),
        module_dir.join(module_name).join("mod.rs"),
    ]
    .into_iter()
    .filter(|path| path.is_file())
    .collect()
}

fn packages_from_workspace(project_workspace: &ProjectWorkspace) -> Vec<RustPackage> {
    match &project_workspace.kind {
        ProjectWorkspaceKind::Cargo { cargo, .. } => {
            let mut packages = cargo
                .packages()
                .map(|package_id| {
                    let package = &cargo[package_id];
                    let targets = package
                        .targets
                        .iter()
                        .map(|target_id| {
                            let target = &cargo[*target_id];
                            RustTarget {
                                name: target.name.clone(),
                                kind: RustTargetKind::from(target.kind),
                                root_file: PathBuf::from(target.root.clone()),
                            }
                        })
                        .collect::<Vec<_>>();

                    let manifest_path = PathBuf::from(AbsPathBuf::from(package.manifest.clone()));
                    let package_root = PathBuf::from(package.manifest.parent().to_path_buf());

                    RustPackage {
                        name: package.name.clone(),
                        manifest_path,
                        package_root,
                        is_workspace_member: package.is_member,
                        targets,
                    }
                })
                .collect::<Vec<_>>();
            packages.sort_by(|left, right| left.manifest_path.cmp(&right.manifest_path));
            packages
        }
        ProjectWorkspaceKind::Json(_) | ProjectWorkspaceKind::DetachedFile { .. } => Vec::new(),
    }
}

fn absolute_path(path: &Path, context: &'static str) -> RustAnalyzerLibResult<PathBuf> {
    path.canonicalize()
        .map_err(|source| RustAnalyzerLibError::io(context, Some(path.to_path_buf()), source))
}

impl From<project_model::TargetKind> for RustTargetKind {
    fn from(value: project_model::TargetKind) -> Self {
        match value {
            project_model::TargetKind::Bin => Self::Bin,
            project_model::TargetKind::Lib { .. } => Self::Lib,
            project_model::TargetKind::Example => Self::Example,
            project_model::TargetKind::Test => Self::Test,
            project_model::TargetKind::Bench => Self::Bench,
            project_model::TargetKind::BuildScript => Self::BuildScript,
            project_model::TargetKind::Other => Self::Other,
        }
    }
}
