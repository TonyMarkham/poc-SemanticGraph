#![allow(deprecated)]

use crate::{
    document_symbols::mapper::range_for_line,
    model::{DocumentSymbolBatchRequest, GraphLanguage, SourceLanguage},
    providers::soul_lsp::SoulLspProvider,
};

use lsp_types::{DocumentSymbol, SymbolKind};
use std::{
    error::Error,
    fs, io,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn maps_soul_documents_and_references() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("documents-and-references")?;
    let source_path = root.join("docs/a.md");
    let target_path = root.join("docs/b.md");
    fs::create_dir_all(root.join("docs"))?;
    fs::write(
        &source_path,
        "\
---
id: feature.a
kind: feature
title: Feature A
---

See [[feature.b|Feature B]].
",
    )?;
    fs::write(
        &target_path,
        "\
---
id: feature.b
kind: feature
title: Feature B
---
",
    )?;

    let provider = SoulLspProvider::new();
    let document_request = DocumentSymbolBatchRequest {
        workspace_root: root.clone(),
        package_path: root.clone(),
        file_paths: vec![source_path.clone(), target_path.clone()],
    };
    let document_symbols = provider.map_document_symbol_items(
        document_request.clone(),
        vec![
            (
                source_path.canonicalize()?,
                vec![document_symbol(
                    "feature.a",
                    Some("Feature A"),
                    Some(vec![reference_symbol("Feature B", "feature.b")]),
                )],
            ),
            (
                target_path.canonicalize()?,
                vec![document_symbol("feature.b", Some("Feature B"), None)],
            ),
        ],
    )?;

    assert_eq!(
        document_symbols.extractions[0].source_file.language,
        SourceLanguage::Markdown
    );
    assert_eq!(
        document_symbols.extractions[0].language,
        GraphLanguage::Soul
    );
    assert!(
        document_symbols.extractions[0]
            .symbols
            .iter()
            .any(|symbol| symbol.name == "feature.a" && symbol.kind == "file")
    );
    assert!(
        document_symbols.extractions[0]
            .symbols
            .iter()
            .any(|symbol| symbol.name == "Feature B" && symbol.kind == "string")
    );

    let targets =
        provider.reference_targets_for_document_symbols(&document_request, &document_symbols)?;
    assert_eq!(targets.len(), 2);
    let canonical_target_path = target_path.canonicalize()?;
    let target = targets
        .iter()
        .find(|target| target.file_path == canonical_target_path)
        .ok_or_else(|| io::Error::other("feature.b target not found"))?;
    let references = provider.map_reference_sets(
        &document_request,
        document_symbols,
        vec![soul_lsp_lib::ResolvedReferenceSet {
            target_file_path: target.file_path.clone(),
            target_selection_range: target.selection_range,
            target_name: target.name.clone(),
            references: vec![soul_lsp_lib::ResolvedReferenceLocation {
                file_path: source_path.canonicalize()?,
                range: range_for_line(6, 4, 27),
            }],
        }],
        targets.len(),
    )?;

    assert_eq!(references.references.len(), 1);
    assert!(
        references.references[0]
            .source_symbol_key
            .contains("name=feature.a")
    );
    assert!(
        references.references[0]
            .target_symbol_key
            .contains("name=feature.b")
    );
    assert_eq!(references.references[0].occurrences.len(), 1);
    assert_eq!(
        references.references[0].raw_json["lsp_method"],
        "textDocument/references"
    );
    Ok(())
}

fn document_symbol(
    name: &str,
    detail: Option<&str>,
    children: Option<Vec<DocumentSymbol>>,
) -> DocumentSymbol {
    DocumentSymbol {
        name: name.to_string(),
        detail: detail.map(ToOwned::to_owned),
        kind: SymbolKind::FILE,
        tags: None,
        deprecated: None,
        range: range_for_line(0, 0, 0),
        selection_range: range_for_line(0, 0, 0),
        children,
    }
}

fn reference_symbol(name: &str, detail: &str) -> DocumentSymbol {
    DocumentSymbol {
        name: name.to_string(),
        detail: Some(detail.to_string()),
        kind: SymbolKind::STRING,
        tags: None,
        deprecated: None,
        range: range_for_line(6, 4, 27),
        selection_range: range_for_line(6, 4, 27),
        children: None,
    }
}

fn temp_dir(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let path = std::env::temp_dir().join(format!(
        "semantic-graph-extract-soul-{name}-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir_all(&path)?;
    Ok(path)
}
