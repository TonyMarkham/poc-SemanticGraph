## Core Workflow

1. Resolve the database path from the configured MCP server or the existing SemanticGraph configuration behavior.
2. Check graph availability with `graph_stats`.
3. Use MCP as the first semantic search path: `graph_search_nodes`, `graph_file_summary`, `graph_route_status`, `graph_neighbors`, or `graph_projection` before broad shell or text search.
4. Fall back to `rg`, `find`, `grep`, `git grep`, or direct file reads only when MCP is unavailable, returns no useful result, route coverage is stale or missing, or exact source text is needed after MCP identifies candidate files.
5. Use node details, edge details, occurrences, and edge evidence to ground findings.
6. Check route freshness when current behavior, recently changed files, or refresh validity matter.
7. Refresh the graph only when the user asked for implementation, refresh, or validation work that requires current graph facts.
8. Cite source files or graph evidence and label uncertain inferences.

## Boundaries

The MCP server is read-only. Do not infer that MCP tools can run extractors, mutate SQLite, expose arbitrary SQL, run shell commands, or read arbitrary files.

Use the progressive references for command boundaries, custom-agent handoffs, local testbed context, and troubleshooting.
