# MCP Tools And Resources

## Read-Only Tools

- `graph_stats`: return database-wide graph counts and latest extraction runs.
- `graph_search_nodes`: search active nodes by label, qualified name, or source path.
- `graph_node_details`: return one node with relation summaries and occurrences.
- `graph_edge_details`: return one edge with endpoints and evidence.
- `graph_projection`: return a bounded active graph projection.
- `graph_neighbors`: return bounded incoming and outgoing active neighbors for a node.
- `graph_shortest_path`: return a bounded active path between two nodes.
- `graph_file_summary`: return symbols, touching edges, and route freshness for a known database file path.
- `graph_route_status`: return route freshness rows filtered by workspace, route, scope, or file.
- `fts_search`: search indexed file contents with Tantivy-backed ranking and SQLite snippets.

## Read-Only Resources

- `semantic-graph://schema`: compact SQLite schema summary.
- `semantic-graph://workspace`: current read-only server context and latest extraction run summaries.
- `semantic-graph://routes`: extractor route names and freshness semantics.
- `semantic-graph://local-testbeds`: local visualization testbed notes.

## Query Guidance

Use `fts_search` first when the request asks which files contain text, searches file contents, names literal terms, asks for case-insensitive text results, wants snippets, or is grep-like. Set `limit` between 1 and 50. If the response has `nextCursor`, call `fts_search` again with that cursor until `nextCursor` is absent or null.

For "which files contain X" or other file-list answers, collect `hits[].path` from every FTS page, deduplicate paths, and answer with the complete MCP-derived path list. Do not use `graph_search_nodes` as a substitute for file-content search. Do not announce, run, recommend, cite, or use `rg`, `find`, `grep`, `git grep`, IDE search, or another shell text search after successful FTS pagination. Do not provide a shell command as a substitute for the requested list.

Use `graph_search_nodes` as the first semantic repo search before broad source inspection when the request names symbols, files, modules, ownership, behavior, or relationships. Use `graph_file_summary`, `graph_route_status`, `graph_neighbors`, and `graph_projection` before falling back to shell or text search for semantic navigation.

Do not start semantic repo search with `rg`, `find`, `grep`, `git grep`, or IDE search. Fall back to those tools only when MCP is unavailable, returns no useful graph/FTS result, route or FTS coverage is stale or missing, MCP has identified candidate files that still need exact source text inspection, or the user explicitly asks for a shell command.

Use `graph_node_details` and `graph_edge_details` when the answer needs occurrences or edge evidence. Use `graph_route_status` or `graph_file_summary` before claiming a route is current.

Use `fts_search` only after FTS has been built with `semantic-graph-extract fts`. It is read-only and never runs extraction or creates missing FTS stores.

The MCP surface is read-only. It does not expose SQL, shell execution, arbitrary file reads, hard deletes, stale closing, reset operations, or extraction tools.
