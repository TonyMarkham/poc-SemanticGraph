use crate::{
    AnalysisWorker, LoadedSoulWorkspace, ResolvedReferenceTarget, SoulLspConfig,
    SoulLspPluginConfig, SoulLspScanConfig, provider_version,
};

use lsp_types::SymbolKind;
use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn live_scan_extracts_document_and_wikilink_symbols_without_index_db() -> Result<(), Box<dyn Error>>
{
    let root = make_workspace("symbols")?;
    let loaded = LoadedSoulWorkspace::load_with_config(&root, SoulLspConfig::default())?;
    let symbols = loaded.document_symbols_for_file(root.join("docs/a.md"))?;

    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].name, "feature.a");
    assert_eq!(symbols[0].kind, SymbolKind::FILE);
    let children = symbols[0]
        .children
        .as_ref()
        .ok_or("document symbol should have reference children")?;
    assert!(children.iter().any(|child| {
        child.name == "Feature B"
            && child.detail.as_deref() == Some("feature.b")
            && child.kind == SymbolKind::STRING
    }));
    assert!(!root.join(".soul/index.db").exists());

    cleanup_workspace(&root);
    Ok(())
}

#[test]
fn live_scan_resolves_references_for_document_symbols() -> Result<(), Box<dyn Error>> {
    let root = make_workspace("references")?;
    let loaded = LoadedSoulWorkspace::load_with_config(&root, SoulLspConfig::default())?;
    let symbols = loaded.document_symbols_for_file(root.join("docs/b.md"))?;
    let target = symbols
        .first()
        .ok_or("document symbol should exist for docs/b.md")?;

    let references = loaded.references_for_symbol(ResolvedReferenceTarget {
        file_path: root.join("docs/b.md"),
        selection_range: target.selection_range,
        name: target.name.clone(),
    })?;

    assert!(references.references.iter().any(|reference| {
        reference.file_path == root.join("docs/a.md") && reference.range.start.line == 6
    }));
    assert!(!root.join(".soul/index.db").exists());

    cleanup_workspace(&root);
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn analysis_worker_serves_live_scan_queries() -> Result<(), Box<dyn Error>> {
    let root = make_workspace("worker")?;
    let worker = AnalysisWorker::start_with_config(&root, SoulLspConfig::default())?;
    let document_symbols = worker
        .document_symbols_for_files(vec![root.join("docs/b.md")])
        .await?;
    let target = document_symbols
        .first()
        .and_then(|(_, symbols)| symbols.first())
        .ok_or("worker should return document symbols")?;
    let references = worker
        .references_for_symbol(ResolvedReferenceTarget {
            file_path: root.join("docs/b.md"),
            selection_range: target.selection_range,
            name: target.name.clone(),
        })
        .await?;
    worker.shutdown().await?;

    assert!(references.references.iter().any(|reference| {
        reference.file_path == root.join("docs/a.md") && reference.range.start.line == 6
    }));
    assert!(!root.join(".soul/index.db").exists());

    cleanup_workspace(&root);
    Ok(())
}

#[test]
fn configured_plugin_scan_discovers_annotated_rust_source_files() -> Result<(), Box<dyn Error>> {
    let Some(plugin_path) = local_soul_plugin_path("rust") else {
        return Ok(());
    };
    let root = make_workspace("plugin-source-discovery")?;
    fs::create_dir_all(root.join("src"))?;
    fs::write(
        root.join("src/backend.rs"),
        r#"use soul_attributes::soul;

#[soul(id = "feature.a")]
pub fn backend() {}
"#,
    )?;
    let config = SoulLspConfig::new(
        SoulLspScanConfig::default(),
        vec![SoulLspPluginConfig::new("rust".to_string(), plugin_path)],
    );

    let loaded = LoadedSoulWorkspace::load_with_config(&root, config)?;
    let source_files = loaded.source_files();
    let symbols = loaded.document_symbols_for_file(root.join("src/backend.rs"))?;

    assert!(source_files.contains(&root.join("src/backend.rs")));
    assert!(symbols.iter().any(|symbol| {
        symbol.name == "feature.a"
            && symbol.detail.as_deref() == Some("rust-attribute")
            && symbol.kind == SymbolKind::OBJECT
    }));
    assert!(!root.join(".soul/index.db").exists());

    cleanup_workspace(&root);
    Ok(())
}

#[test]
fn reports_live_scan_provider_version() {
    assert!(provider_version().unwrap_or_default().contains("live scan"));
}

fn local_soul_plugin_path(language: &str) -> Option<PathBuf> {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent()?.parent()?;
    let path = repo_root
        .join(".soul/plugins")
        .join(format!("{language}{}", std::env::consts::DLL_SUFFIX));
    path.is_file().then_some(path)
}

fn make_workspace(label: &str) -> Result<PathBuf, Box<dyn Error>> {
    let root = std::env::temp_dir().join(format!(
        "soul-lsp-lib-{label}-{}-{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    ));
    cleanup_workspace(&root);

    fs::create_dir_all(root.join("docs"))?;
    fs::write(
        root.join("docs/a.md"),
        r#"---
id: feature.a
kind: feature
title: Feature A
---

See [[feature.b|Feature B]] from here.
"#,
    )?;
    fs::write(
        root.join("docs/b.md"),
        r#"---
id: feature.b
kind: feature
title: Feature B
---
"#,
    )?;

    Ok(root)
}

fn cleanup_workspace(root: &Path) {
    if root.exists() {
        let _remove_result = fs::remove_dir_all(root);
    }
}
