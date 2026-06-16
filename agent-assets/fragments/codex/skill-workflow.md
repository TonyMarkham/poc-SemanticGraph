## Core Workflow

1. Resolve the database path from the configured MCP server or the existing SemanticGraph configuration behavior.
2. Check graph availability with `graph_stats`.
3. Search relevant nodes before broad source inspection when the task is about relationships, references, calls, provenance, route freshness, query surfaces, or graph refresh.
4. Use node details, edge details, occurrences, and edge evidence to ground findings.
5. Check route freshness when current behavior, recently changed files, or refresh validity matter.
6. Refresh the graph only when the user asked for implementation, refresh, or validation work that requires current graph facts.
7. Cite source files or graph evidence and label uncertain inferences.

## Boundaries

The MCP server is read-only. Do not infer that MCP tools can run extractors, mutate SQLite, expose arbitrary SQL, run shell commands, or read arbitrary files.

Use the progressive references for command boundaries, custom-agent handoffs, local testbed context, and troubleshooting.
