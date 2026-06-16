---
name: semantic-graph
description: "Use SemanticGraph MCP tools as the first semantic repo search and code-navigation surface for files, symbols, modules, relationships, references, call graphs, provenance, route freshness, and graph refresh planning work."
---

# SemanticGraph

## When To Use This Skill

Use this skill when the user's task needs semantic repo search, code navigation, source ownership, symbol or file discovery, code relationships, references, call graphs, source provenance, evidence, route freshness, stale graph facts, SemanticGraph MCP query work, or planning and validating SemanticGraph refreshes.

For semantic repo search, the first search choice must always be the SemanticGraph MCP server. Use shell or text search tools such as `rg`, `find`, `grep`, `git grep`, or IDE search only after MCP is unavailable, returns no useful graph result, has stale or missing route coverage, or identifies candidate files that still need exact source text inspection.

If you fall back from MCP to shell or text search, state the fallback reason in the work notes or final answer.

## SemanticGraph Facts

SemanticGraph stores canonical graph data in SQLite `nodes` and `edges`, with source proof in `occurrences` and `edge_evidence`. Directed edges are first-class. Confidence values are `EXTRACTED`, `INFERRED`, and `AMBIGUOUS`.

Route freshness is tracked separately from graph rows. A node or edge can still exist while a route is stale, so use route status when current behavior or recently changed files matter.

Stale graph state is soft-closed. Treat stale rows as historical evidence unless the user explicitly asks about history; prefer active rows for current code questions.

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

## Progressive References

- `references/mcp-tools.md` - MCP Tools And Resources
- `references/rust-extraction.md` - Rust Extraction
- `references/csharp-extraction.md` - CSharp Extraction
- `references/local-testbeds.md` - Local Testbeds
- `references/agent-handoffs.md` - Agent Handoffs
- `references/troubleshooting.md` - Troubleshooting
