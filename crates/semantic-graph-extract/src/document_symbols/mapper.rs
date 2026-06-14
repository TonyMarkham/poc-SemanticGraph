use crate::document_symbols::paths::percent_encode_component;

use semantic_graph_store::TextRange;

use lsp_types::{Position, Range, SymbolKind};

pub fn text_range_from_lsp(range: Range) -> TextRange {
    TextRange {
        start_line: i64::from(range.start.line),
        start_col: i64::from(range.start.character),
        end_line: i64::from(range.end.line),
        end_col: i64::from(range.end.character),
    }
}

pub fn build_symbol_key(
    file_uri: &str,
    kind: &str,
    selection_range: TextRange,
    name: &str,
    parent_path: &[String],
) -> String {
    let parent = parent_path.join("::");

    format!(
        "{file_uri}#kind={};selection={}:{}-{}:{};name={};parent={}",
        percent_encode_component(kind),
        selection_range.start_line,
        selection_range.start_col,
        selection_range.end_line,
        selection_range.end_col,
        percent_encode_component(name),
        percent_encode_component(&parent)
    )
}

pub fn qualified_name(parent_path: &[String], name: &str) -> String {
    if parent_path.is_empty() {
        name.to_string()
    } else {
        format!("{}::{name}", parent_path.join("::"))
    }
}

pub fn normalize_symbol_kind(kind: SymbolKind) -> &'static str {
    if kind == SymbolKind::FILE {
        "file"
    } else if kind == SymbolKind::MODULE || kind == SymbolKind::NAMESPACE {
        "module"
    } else if kind == SymbolKind::PACKAGE {
        "package"
    } else if kind == SymbolKind::CLASS {
        "class"
    } else if kind == SymbolKind::METHOD {
        "method"
    } else if kind == SymbolKind::PROPERTY {
        "property"
    } else if kind == SymbolKind::FIELD {
        "field"
    } else if kind == SymbolKind::CONSTRUCTOR {
        "constructor"
    } else if kind == SymbolKind::ENUM {
        "enum"
    } else if kind == SymbolKind::INTERFACE {
        "interface"
    } else if kind == SymbolKind::FUNCTION {
        "function"
    } else if kind == SymbolKind::VARIABLE {
        "variable"
    } else if kind == SymbolKind::CONSTANT {
        "constant"
    } else if kind == SymbolKind::STRING {
        "string"
    } else if kind == SymbolKind::NUMBER {
        "number"
    } else if kind == SymbolKind::BOOLEAN {
        "boolean"
    } else if kind == SymbolKind::ARRAY {
        "array"
    } else if kind == SymbolKind::OBJECT {
        "object"
    } else if kind == SymbolKind::KEY {
        "key"
    } else if kind == SymbolKind::NULL {
        "null"
    } else if kind == SymbolKind::ENUM_MEMBER {
        "enum_member"
    } else if kind == SymbolKind::STRUCT {
        "struct"
    } else if kind == SymbolKind::EVENT {
        "event"
    } else if kind == SymbolKind::OPERATOR {
        "operator"
    } else if kind == SymbolKind::TYPE_PARAMETER {
        "type_parameter"
    } else {
        "object"
    }
}

pub fn lsp_symbol_kind_name(kind: SymbolKind) -> &'static str {
    if kind == SymbolKind::FILE {
        "File"
    } else if kind == SymbolKind::MODULE {
        "Module"
    } else if kind == SymbolKind::NAMESPACE {
        "Namespace"
    } else if kind == SymbolKind::PACKAGE {
        "Package"
    } else if kind == SymbolKind::CLASS {
        "Class"
    } else if kind == SymbolKind::METHOD {
        "Method"
    } else if kind == SymbolKind::PROPERTY {
        "Property"
    } else if kind == SymbolKind::FIELD {
        "Field"
    } else if kind == SymbolKind::CONSTRUCTOR {
        "Constructor"
    } else if kind == SymbolKind::ENUM {
        "Enum"
    } else if kind == SymbolKind::INTERFACE {
        "Interface"
    } else if kind == SymbolKind::FUNCTION {
        "Function"
    } else if kind == SymbolKind::VARIABLE {
        "Variable"
    } else if kind == SymbolKind::CONSTANT {
        "Constant"
    } else if kind == SymbolKind::STRING {
        "String"
    } else if kind == SymbolKind::NUMBER {
        "Number"
    } else if kind == SymbolKind::BOOLEAN {
        "Boolean"
    } else if kind == SymbolKind::ARRAY {
        "Array"
    } else if kind == SymbolKind::OBJECT {
        "Object"
    } else if kind == SymbolKind::KEY {
        "Key"
    } else if kind == SymbolKind::NULL {
        "Null"
    } else if kind == SymbolKind::ENUM_MEMBER {
        "EnumMember"
    } else if kind == SymbolKind::STRUCT {
        "Struct"
    } else if kind == SymbolKind::EVENT {
        "Event"
    } else if kind == SymbolKind::OPERATOR {
        "Operator"
    } else if kind == SymbolKind::TYPE_PARAMETER {
        "TypeParameter"
    } else {
        "Unknown"
    }
}

pub fn range_for_line(line: u32, start: u32, end: u32) -> Range {
    Range {
        start: Position {
            line,
            character: start,
        },
        end: Position {
            line,
            character: end,
        },
    }
}
