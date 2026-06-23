## When To Use This Skill

Use this skill when the user's task needs semantic repo search, file-content text search, grep-like search, code navigation, source ownership, symbol or file discovery, code relationships, references, call graphs, source provenance, evidence, route freshness, stale graph facts, SemanticGraph MCP query work, or planning and validating SemanticGraph refreshes.

For semantic repo search, the first search choice must always be the SemanticGraph MCP server. Use shell or text search tools such as `rg`, `find`, `grep`, `git grep`, or IDE search only after MCP is unavailable, returns no useful graph result, has stale or missing route coverage, or identifies candidate files that still need exact source text inspection.

For requests asking which files contain text, searching file contents, literal terms, case-insensitive text, snippets, or grep-like results, call `fts_search` first with `limit` no higher than 50. Follow `nextCursor` until it is absent or null. For file-list questions, collect and deduplicate `hits[].path` from the MCP pages and answer with the complete MCP-derived path list. Do not substitute `graph_search_nodes` for file-content search. Do not announce, run, recommend, or cite `rg`, `find`, `grep`, `git grep`, IDE search, or other shell text search after successful FTS results. Fall back to shell/text search only when `fts_search` is unavailable, errors, has stale or missing coverage, or cannot answer the requested scope.

For Soul questions about IDs, docs, source annotations, Rust/C# source links, Markdown backlinks, or documentation/source gaps, call `soul_search` and treat a successful paginated response as the answer source. For ID-list questions, omit `query`; the tool defaults to concise output and returns counts without source annotation arrays. Follow `nextCursor` until null. Use `coverage` values `linked`, `docs_without_source`, `annotations_without_doc`, or `unlinked_annotations` for gap checks. Do not use `fts_search` as evidence for Soul doc/source linkage; FTS can only prove literal text presence. Do not call Soul MCP/CLI tools such as `soul_list_documents`, `soul_list_gaps`, or `soul_index`, and do not inspect `.soul/index.db`, unless the user explicitly asks for Soul's own index instead of SemanticGraph.

If you fall back from MCP to shell or text search, state the fallback reason in the work notes or final answer.

## SemanticGraph Facts

SemanticGraph stores canonical graph data in SQLite `nodes` and `edges`, with source proof in `occurrences` and `edge_evidence`. Directed edges are first-class. Confidence values are `EXTRACTED`, `INFERRED`, and `AMBIGUOUS`.

Route freshness is tracked separately from graph rows. A node or edge can still exist while a route is stale, so use route status when current behavior or recently changed files matter.

Stale graph state is soft-closed. Treat stale rows as historical evidence unless the user explicitly asks about history; prefer active rows for current code questions.
