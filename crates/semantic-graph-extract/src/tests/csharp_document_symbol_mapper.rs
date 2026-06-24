#![allow(deprecated)]

use crate::{
    document_symbols::mapper::range_for_line,
    model::{DocumentSymbolBatchRequest, DocumentSymbolRequest, SourceLanguage},
    providers::csharp_ls::{CSharpDocumentSymbolMapper, CSharpLsProvider},
};

use lsp_types::{DocumentSymbol, DocumentSymbolResponse, SymbolKind};
use serde_json::json;
use std::{
    error::Error,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn unwraps_csharp_file_root_document_symbol() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("unwraps-file-root")?;
    let source_path = root.join("Program.cs");
    fs::write(
        &source_path,
        "namespace Demo { class Program { void Run() {} } }",
    )?;
    let symbols = vec![DocumentSymbol {
        name: "Program.cs".to_string(),
        detail: None,
        kind: SymbolKind::FILE,
        tags: None,
        deprecated: None,
        range: range_for_line(0, 0, 48),
        selection_range: range_for_line(0, 0, 48),
        children: Some(vec![DocumentSymbol {
            name: "Demo".to_string(),
            detail: None,
            kind: SymbolKind::NAMESPACE,
            tags: None,
            deprecated: None,
            range: range_for_line(0, 0, 48),
            selection_range: range_for_line(0, 10, 14),
            children: None,
        }]),
    }];

    let extraction = CSharpDocumentSymbolMapper::map_response(
        DocumentSymbolRequest {
            workspace_root: root.clone(),
            package_path: root.clone(),
            file_path: source_path,
        },
        DocumentSymbolResponse::Nested(symbols),
        None,
        json!({}),
    )?;

    assert_eq!(extraction.source_file.language, SourceLanguage::CSharp);
    assert_eq!(extraction.symbols.len(), 1);
    assert_eq!(extraction.symbols[0].name, "Demo");
    assert_eq!(extraction.symbols[0].parent_symbol_key, None);
    assert_eq!(extraction.relations.len(), 1);
    assert_eq!(
        extraction.relations[0].source_symbol_key,
        extraction.source_file.file_symbol_key
    );
    Ok(())
}

#[test]
fn maps_incoming_calls_as_caller_to_callee_edges() -> Result<(), Box<dyn Error>> {
    let root = temp_dir("incoming-calls")?;
    let caller_path = root.join("Caller.cs");
    let callee_path = root.join("Callee.cs");
    fs::write(
        &caller_path,
        "class Caller { void Run() { new Callee().Go(); } }",
    )?;
    fs::write(&callee_path, "class Callee { public void Go() {} }")?;
    let provider = CSharpLsProvider::new();
    let document_request = DocumentSymbolBatchRequest {
        workspace_root: root.clone(),
        package_path: root.clone(),
        file_paths: vec![caller_path.clone(), callee_path.clone()],
    };
    let document_symbols = provider.map_document_symbol_items(
        document_request.clone(),
        vec![
            (
                caller_path.canonicalize()?,
                vec![method_symbol("Run", SymbolKind::METHOD, 0, 20, 23, 34, 48)],
            ),
            (
                callee_path.canonicalize()?,
                vec![method_symbol("Go", SymbolKind::METHOD, 0, 27, 29, 15, 34)],
            ),
        ],
    )?;
    let call_targets =
        provider.call_targets_for_document_symbols(&document_request, &document_symbols)?;
    let canonical_callee_path = callee_path.canonicalize()?;
    let callee_target = call_targets
        .iter()
        .find(|target| target.file_path == canonical_callee_path)
        .ok_or("expected callee target")?
        .clone();
    let incoming = csharp_ls_lib::ResolvedIncomingCallSet {
        target_file_path: callee_target.file_path.clone(),
        target_selection_range: callee_target.selection_range,
        incoming_calls: vec![csharp_ls_lib::ResolvedIncomingCall {
            caller_file_path: caller_path.canonicalize()?,
            caller_name: "Run".to_string(),
            caller_kind: "METHOD".to_string(),
            caller_range: range_for_line(0, 15, 48),
            caller_selection_range: range_for_line(0, 20, 23),
            from_ranges: vec![range_for_line(0, 34, 48)],
        }],
        skipped_non_callable_prepare_items: 0,
    };

    let calls = provider.map_incoming_call_sets(
        &document_request,
        document_symbols,
        vec![incoming],
        call_targets.len(),
    )?;

    assert_eq!(calls.calls.len(), 1);
    assert!(calls.calls[0].caller_symbol_key.contains("name=Run"));
    assert!(calls.calls[0].callee_symbol_key.contains("name=Go"));
    assert_eq!(calls.calls[0].occurrences.len(), 1);
    assert_eq!(
        calls.calls[0].raw_json["lsp_method"],
        "callHierarchy/incomingCalls"
    );
    Ok(())
}

fn method_symbol(
    name: &str,
    kind: SymbolKind,
    line: u32,
    selection_start: u32,
    selection_end: u32,
    range_start: u32,
    range_end: u32,
) -> DocumentSymbol {
    DocumentSymbol {
        name: name.to_string(),
        detail: None,
        kind,
        tags: None,
        deprecated: None,
        range: range_for_line(line, range_start, range_end),
        selection_range: range_for_line(line, selection_start, selection_end),
        children: None,
    }
}

fn temp_dir(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let path = std::env::temp_dir().join(format!(
        "semantic-graph-extract-csharp-{name}-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir_all(&path)?;
    Ok(path)
}
