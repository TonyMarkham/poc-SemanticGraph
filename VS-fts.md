# Add Full Text Search To The MCP Server

## Context

`semantic-graph-extract fts` now writes a read model for file content:

- SQLite stores FTS document metadata and original UTF-8 text in
  `fts_documents` and `fts_document_contents`.
- Tantivy sidecar: `fts.db` maps to `fts.tantivy` through `db.with_extension("tantivy")`.
- Tantivy indexes the searchable content fields and stores hit metadata:
  `uri`, `path`, `language`, and `content_hash`.
- Route status: the extractor records `fts.full_text` at workspace scope.
- Current README still says text query and MCP integration are later work.

The MCP server is intentionally read-only. It should query an already-built FTS
database and sidecar, but it should not run `semantic-graph-extract fts`, create
missing stores, or mutate graph/FTS data.

## Goal

Expose indexed file-content search through the stdio MCP server while preserving
the current graph tools and config behavior.

Initial MCP surface:

- New tool: `fts_search`
- Required input: `query`
- Optional inputs: `limit`, `language`, `pathPrefix`, `caseSensitive`,
  `contextLines`, `cursor`
- Output: applied/requested limit, FTS DB path, Tantivy index path, and ranked
  hits with URI, path, language, content hash, score, line range, snippets, and
  `nextCursor` when another page is available.

For FTS, Tantivy is authoritative for search membership, ranking, and
pagination. SQLite hydrates full content for snippets and remains authoritative
for semantic graph data exposed through the existing graph tools.

## Non-Goals

- Do not make MCP run extraction or refresh FTS.
- Do not merge file-content FTS into `graph_search_nodes`.
- Do not expose arbitrary SQL.
- Do not require the visualizer to change in this pass.
- Do not reintroduce the dropped SQLite FTS5 table.

## Implementation Plan

1. Resolve FTS paths for the MCP server.

   - Extend `ServerArgs` in
     `crates/semantic-graph-mcp-server/src/args/server_args.rs` with
     `--fts-database-path`.
   - Extend `ResolvedServerConfig` with `fts_database_path: Option<PathBuf>` and
     `fts_index_path: Option<PathBuf>`.
   - Config-based startup should resolve `[fts].db_path` relative to the
     discovered workspace root. If `[fts].db_path` is unset, fall back to the
     resolved graph database path to match extractor behavior.
   - Explicit `--fts-database-path` should override config for FTS only.
   - Keep `--database-path` as a graph database override. When it bypasses
     config and no `--fts-database-path` is supplied, either use the same path as
     the FTS fallback or return a clear setup error; choose one behavior and
     cover it in tests.

2. Add a Tantivy search API.

   - In `crates/semantic-graph-search-tantivy`, add request/result types for
     search hits instead of only `count_case_insensitive_candidates`.
   - Add a read-only `search` method that opens an index reader, chooses
     `content_ci` or `content_cs`, runs a bounded `TopDocs` query, and returns
     stored URI/path/language/content_hash plus score.
   - Preserve deterministic pagination by ordering by Tantivy score with a
     stable tie-breaker such as URI and content hash.
   - Do not create an index directory on read. Keep `open_or_create` for the
     extractor, and add a separate read-only open path for MCP/query use.

3. Add an FTS query service.

   - In `crates/semantic-graph-query`, add one-type-per-file models:
     `FtsSearchRequest`, `FtsSearchResults`, `FtsSearchHit`, and snippet/line
     DTOs.
   - Add a service method or small `FtsQueryService` that owns the FTS SQLite DB
     path, Tantivy index path, and `QueryServiceConfig`.
   - Reuse existing limit validation style and add a max FTS search limit if
     the existing `max_search_limit` is not appropriate.
   - Page directly over Tantivy results. Do not over-fetch solely to compensate
     for SQLite filtering, because SQLite is not the FTS membership authority.
   - For each Tantivy hit, query SQLite read-only by URI/content hash to hydrate
     `fts_document_contents.content` for snippets.
   - Generate snippets from `fts_document_contents.content`; keep snippets
     bounded by `contextLines` and avoid returning whole files by default.
   - If Tantivy returns a hit that SQLite cannot hydrate, surface a clear
     consistency diagnostic instead of silently changing membership or ranking.
   - Map Tantivy errors into the existing typed error pattern. Do not add
     `anyhow`.

4. Wire the MCP tool.

   - Add `crates/semantic-graph-mcp-server/src/tools/search_text.rs` with
     `FtsSearchParams` and `From<FtsSearchParams>`.
   - Add `FTS_SEARCH` to `tool_registry.rs`, include schema generation, and
     keep annotations read-only, non-destructive, closed-world.
   - Add a `call_tool` branch in `server/mcp_server.rs`.
   - Extend `ServerState` to hold the FTS query service or explicit optional FTS
     paths.
   - Return a clear MCP invalid-params/setup error when the configured FTS DB or
     Tantivy index is missing.

5. Update docs and generated agent assets.

   - Update README's FTS section to remove "MCP integration is later work" once
     the tool exists.
   - Add `fts_search` to `agent-assets/fragments/common/mcp-tools.md` and any
     expected generated asset snapshots.
   - Consider adding an `semantic-graph://fts` resource with FTS DB path,
     Tantivy path, active document count, and latest `fts.full_text` route
     status. Keep it read-only and informational.

6. Test the behavior.

   - Unit-test FTS path resolution in `server_args.rs`.
   - Unit-test Tantivy hit retrieval in `semantic-graph-search-tantivy`.
   - Unit-test query hydration so Tantivy hit ordering is preserved while SQLite
     provides snippets and consistency diagnostics.
   - Extend `crates/semantic-graph-mcp-server/tests/stdio.rs` to seed an FTS DB,
     write a Tantivy sidecar, list `fts_search`, call it, and verify snippets.
   - Verify missing FTS DB/index errors are explicit and do not prevent the
     existing graph tools from working unless startup resolution intentionally
     requires FTS.

## Validation Commands

Run the most specific practical checks for the implementation:

```sh
cargo test -p semantic-graph-search-tantivy
cargo test -p semantic-graph-query
cargo test -p semantic-graph-mcp-server
SQLX_OFFLINE=true cargo test -p semantic-graph-extract fts
just confidence
```

If agent assets are regenerated, also run the crate tests that validate their
expected output.

## Definition Of Done

- `fts_search` is available from MCP, is read-only, and returns bounded,
  paginated file-content hits whose membership and ranking come from Tantivy.
- SQLite hydration provides snippets without changing Tantivy membership,
  ranking, or pagination.
- FTS path resolution is documented, tested, and handles both config discovery
  and explicit override paths.
- README and generated agent assets describe the new MCP tool accurately.
- The targeted Rust tests for Tantivy search, query hydration, pagination, MCP
  stdio, and FTS extraction pass.
- `just confidence` passes without errors or warnings.

## Open Decisions

- Should `fts_search` be listed unconditionally and fail only when called, or
  should the server omit it when no FTS path is configured?
- Should `--database-path` imply the same SQLite file for FTS when no
  `--fts-database-path` is supplied, or should FTS require config/explicit path
  in override mode?
- Should v1 expose only literal text search, or allow Tantivy query syntax?
- Should snippets include byte offsets, line/column ranges, or line numbers
  only?
- Should a Tantivy hit that cannot be hydrated from SQLite return metadata only,
  or fail with a consistency error?
