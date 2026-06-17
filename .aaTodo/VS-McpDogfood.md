# MCP Dogfooding Items

Dogfooding notes from using the SemanticGraph MCP server to find Rust files with
more than one type declaration.

## Process Fixes

- Treat "use the MCP" as an interface constraint, not an implementation hint.
- Do not fall back to direct SQLite access when the user requested MCP-only work.
- If the MCP cannot express a complete query directly, stop and say that before
  using another access path.
- Make agents report when they are using local file enumeration only as
  candidate input, with MCP responses as the source of truth.

## MCP Product Fixes

- Add a first-class MCP tool for type-declaration audits, such as
  `graph_type_declaration_violations`.
- Add pagination or cursor support to file-oriented MCP tools, especially
  `graph_route_status`.
- Add a MCP file/symbol query that supports filters for language, symbol kind,
  active state, and grouping by file.
- Centralize the graph's "type declaration kind" definition so clients do not
  reinvent whether Rust traits are `interface`, Rust impl blocks are `object`,
  and so on.
- Return machine-shaped audit results directly for checks like "one type per
  file" instead of forcing clients to issue hundreds of `graph_file_summary`
  calls.
