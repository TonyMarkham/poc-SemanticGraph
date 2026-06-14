use std::error::Error;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::{
    document_symbols_for_file, load_workspace, package_source_files, provider_version,
    workspace_source_files,
};
use lsp_types::DocumentSymbol;

static WORKSPACE_LOAD_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn loads_workspace_members_and_source_files() -> Result<(), Box<dyn Error>> {
    let _guard = workspace_load_guard()?;
    let repo_root = repo_root()?;
    let model = load_workspace(&repo_root)?;
    let mut member_names = model
        .packages
        .iter()
        .filter(|package| package.is_workspace_member)
        .map(|package| package.name.as_str())
        .collect::<Vec<_>>();
    member_names.sort();

    assert!(member_names.contains(&"rust-analyzer-lib"));
    assert!(member_names.contains(&"semantic-graph-extract"));
    assert!(member_names.contains(&"semantic-graph-store"));
    assert!(member_names.contains(&"wip"));

    let source_files = workspace_source_files(&model);
    assert!(source_files.iter().any(|path| {
        relative_path(&repo_root, path).as_deref() == Some("crates/wip/src/lib.rs")
    }));
    assert!(source_files.iter().all(|path| {
        !relative_path(&repo_root, path)
            .unwrap_or_default()
            .starts_with("submodules/")
    }));

    Ok(())
}

#[test]
fn discovers_wip_package_source_files() -> Result<(), Box<dyn Error>> {
    let _guard = workspace_load_guard()?;
    let repo_root = repo_root()?;
    let package_path = repo_root.join("crates/wip");
    let model = load_workspace(&repo_root)?;
    let files = package_source_files(&model, &package_path)
        .into_iter()
        .map(|path| relative_path(&repo_root, &path).unwrap_or_default())
        .collect::<Vec<_>>();

    assert_eq!(
        files,
        vec![
            "crates/wip/src/lib.rs".to_string(),
            "crates/wip/src/models.rs".to_string(),
            "crates/wip/src/pipeline.rs".to_string(),
            "crates/wip/src/tests/mod.rs".to_string(),
        ]
    );
    Ok(())
}

#[test]
fn extracts_document_symbols_for_wip_test_module() -> Result<(), Box<dyn Error>> {
    let _guard = workspace_load_guard()?;
    let repo_root = repo_root()?;
    let symbols =
        document_symbols_for_file(&repo_root, repo_root.join("crates/wip/src/tests/mod.rs"))?;
    let mut names = Vec::new();
    collect_symbol_names(&symbols, &mut names);

    assert!(
        names
            .iter()
            .any(|name| name == "processor_tracks_active_widgets")
    );
    Ok(())
}

#[test]
fn provider_version_is_deterministic_without_binary_probe() {
    assert_eq!(
        provider_version(),
        Some("rust-analyzer-lib 0.1.0 using pinned rust-analyzer submodule".to_string())
    );
}

fn repo_root() -> Result<PathBuf, Box<dyn Error>> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let crates_dir = manifest_dir
        .parent()
        .ok_or_else(|| io::Error::other("rust-analyzer-lib manifest dir has no parent"))?;
    let repo_root = crates_dir
        .parent()
        .ok_or_else(|| io::Error::other("crates directory has no parent"))?;

    Ok(repo_root.to_path_buf())
}

fn workspace_load_guard() -> Result<std::sync::MutexGuard<'static, ()>, Box<dyn Error>> {
    WORKSPACE_LOAD_LOCK
        .lock()
        .map_err(|_| io::Error::other("workspace load test mutex was poisoned").into())
}

fn relative_path(root: &Path, path: &Path) -> Option<String> {
    path.strip_prefix(root)
        .ok()
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
}

fn collect_symbol_names(symbols: &[DocumentSymbol], names: &mut Vec<String>) {
    for symbol in symbols {
        names.push(symbol.name.clone());
        if let Some(children) = &symbol.children {
            collect_symbol_names(children, names);
        }
    }
}
