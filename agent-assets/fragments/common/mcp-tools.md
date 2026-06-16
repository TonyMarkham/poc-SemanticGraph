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

## Read-Only Resources

- `semantic-graph://schema`: compact SQLite schema summary.
- `semantic-graph://workspace`: current read-only server context and latest extraction run summaries.
- `semantic-graph://routes`: extractor route names and freshness semantics.
- `semantic-graph://local-testbeds`: local visualization testbed notes.

## Query Guidance

Use `graph_search_nodes` before broad source inspection when the request names symbols, files, modules, or relationships. Use `graph_node_details` and `graph_edge_details` when the answer needs occurrences or edge evidence. Use `graph_route_status` or `graph_file_summary` before claiming a route is current.

The MCP surface is read-only. It does not expose SQL, shell execution, arbitrary file reads, hard deletes, stale closing, reset operations, or extraction tools.
