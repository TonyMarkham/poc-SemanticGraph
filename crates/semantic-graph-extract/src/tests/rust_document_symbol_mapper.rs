use std::env;
use std::error::Error;

use crate::error::ExtractError;
use crate::model::DocumentSymbolRequest;
use crate::providers::rust_analyzer::RustDocumentSymbolMapper;
use lsp_types::DocumentSymbolResponse;
use serde_json::json;

#[test]
fn rejects_flat_symbol_information_response() -> std::result::Result<(), Box<dyn Error>> {
    let value = serde_json::json!([
        {
            "name": "flat",
            "kind": 12,
            "location": {
                "uri": "file:///tmp/lib.rs",
                "range": {
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": 0, "character": 4 }
                }
            }
        }
    ]);
    let response: DocumentSymbolResponse = serde_json::from_value(value)?;
    let cwd = env::current_dir()?;

    let result = RustDocumentSymbolMapper::map_response(
        DocumentSymbolRequest {
            workspace_root: cwd.clone(),
            package_path: cwd.join("crates/wip"),
            file_path: cwd.join("crates/wip/src/lib.rs"),
        },
        response,
        None,
        json!({}),
    );

    assert!(matches!(result, Err(ExtractError::ResponseShape { .. })));
    Ok(())
}
