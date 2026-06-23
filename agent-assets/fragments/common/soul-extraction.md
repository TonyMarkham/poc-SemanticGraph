## Commands

Soul refreshes use `semantic-graph-extract` and the in-process `soul-lsp-lib`
facade over the checked-in Soul submodule. They do not shell out to `soul-lsp`
and do not read or write Soul's `.soul/index.db`.

- `soul-file`: refresh one Soul-backed document or annotation source file.
- `soul-workspace`: refresh selected routes for the workspace.

`soul-file` supports one route selector at a time: `--symbols` or
`--references`. With no selector, it refreshes symbols and references.

`soul-workspace` supports combinable `--symbols` and `--references` selectors.
With no selector, it refreshes symbols and references. Relation-only
`--references` runs require the selected files' symbol graph to already exist
unless `--symbols` is selected in the same invocation.

## Routes

- `soul.document_symbols`
- `soul.references`

Soul extraction reads `[soul]` from `.refactor-radar/config.toml`, loads
configured annotation plugins, and runs `indexer::scan_repository` in memory
through `soul-lsp-lib`. The extractor must not read `.soul/soul.toml` for this
path.

Calls are not currently supported because Soul LSP has no call hierarchy route.

## MCP Query

Use `soul_search` for stored Soul graph lookup. It searches SemanticGraph
SQLite graph rows by Soul ID, node name, qualified name, doc path, or source
path, then returns the matching Soul document node, Rust/C# source annotation
nodes, Markdown references, and gaps. It does not call Soul, read
`.soul/index.db`, or use FTS. Omit `query` or pass a blank query to list all
indexed Soul IDs. ID-list requests default to concise output and return counts
without source annotation arrays or Markdown references unless
`includeSourceAnnotations` or `includeMarkdownSources` is explicitly `true`.
Follow `nextCursor` for paging. Use `coverage` values `linked`,
`docs_without_source`, `annotations_without_doc`, or `unlinked_annotations` to
ask coverage questions directly.

Do not call Soul MCP/CLI tools such as `soul_list_documents`, `soul_list_gaps`,
or `soul_index`, and do not query `.soul/index.db`, after `soul_search`
succeeds unless the user explicitly asks for Soul's own index instead of
SemanticGraph.
