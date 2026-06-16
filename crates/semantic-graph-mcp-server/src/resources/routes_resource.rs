pub const ROUTES_RESOURCE_URI: &str = "semantic-graph://routes";

pub fn routes_resource_text() -> String {
    [
        "SemanticGraph route freshness semantics.",
        "",
        "Current extractor route names are:",
        "- rust.document_symbols",
        "- rust.references",
        "- rust.calls",
        "- csharp.document_symbols",
        "- csharp.references",
        "- csharp.calls",
        "",
        "Scopes are file and workspace. File-scoped statuses use the database file path or URI as the scope key. Workspace-scoped statuses use the workspace root URI.",
        "The query tools return stored route values exactly as recorded in SQLite.",
    ]
    .join("\n")
}
