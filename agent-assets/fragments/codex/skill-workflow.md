## Core Workflow

1. Resolve the database path from the configured MCP server or the existing SemanticGraph configuration behavior.
2. For file-content or grep-like text requests, call `fts_search` directly; do not call `graph_stats` as a preflight for text search.
3. Use `limit <= 50`, follow `nextCursor` until exhausted, and deduplicate `hits[].path` for file-list answers.
4. Answer file-list requests with the complete MCP-derived path list. Do not provide a shell command as a substitute for the list.
5. For symbol, file, module, ownership, behavior, relationship, reference, or call-graph requests, check graph availability with `graph_stats`, then use MCP graph tools first: `graph_search_nodes`, `graph_file_summary`, `graph_route_status`, `graph_neighbors`, or `graph_projection`.
6. Fall back to `rg`, `find`, `grep`, `git grep`, or direct file reads only when the relevant MCP search is unavailable, returns no useful result, route/FTS coverage is stale or missing, or exact source text is needed after MCP identifies candidate files. Do not announce, run, recommend, cite, or use shell search to verify a successful paginated `fts_search` file-list answer.
7. Use node details, edge details, occurrences, and edge evidence to ground findings.
8. Check route freshness when current behavior, recently changed files, or refresh validity matter.
9. Refresh the graph only when the user asked for implementation, refresh, or validation work that requires current graph facts.
10. Cite source files or graph evidence and label uncertain inferences.

## Boundaries

The MCP server is read-only. Do not infer that MCP tools can run extractors, mutate SQLite, expose arbitrary SQL, run shell commands, or read arbitrary files.

Use the progressive references for command boundaries, custom-agent handoffs, local testbed context, and troubleshooting.
