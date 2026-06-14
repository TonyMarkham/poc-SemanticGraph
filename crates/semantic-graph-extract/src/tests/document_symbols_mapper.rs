use crate::document_symbols::mapper::build_symbol_key;
use semantic_graph_store::TextRange;

#[test]
fn symbol_keys_include_parent_path() {
    let key = build_symbol_key(
        "file:///repo/src/lib.rs",
        "method",
        TextRange {
            start_line: 10,
            start_col: 8,
            end_line: 10,
            end_col: 14,
        },
        "render",
        &["Widget".to_string()],
    );

    assert_eq!(
        key,
        "file:///repo/src/lib.rs#kind=method;selection=10:8-10:14;name=render;parent=Widget"
    );
}
