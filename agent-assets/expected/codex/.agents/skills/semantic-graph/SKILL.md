---
name: semantic-graph
description: "Use SemanticGraph MCP tools and extraction route knowledge for code relationship, reference, call graph, provenance, route freshness, and graph refresh planning work."
---

# SemanticGraph

## When To Use This Skill

Use this skill when the user's task is about code relationships, references, call graphs, source provenance, evidence, route freshness, stale graph facts, SemanticGraph MCP query work, or planning and validating SemanticGraph refreshes.

Do not claim every task must query the graph first. For unrelated formatting, copy edits, build failures, or ordinary local code changes, use the graph only when relationship, provenance, freshness, or graph-refresh facts matter.

## SemanticGraph Facts

SemanticGraph stores canonical graph data in SQLite `nodes` and `edges`, with source proof in `occurrences` and `edge_evidence`. Directed edges are first-class. Confidence values are `EXTRACTED`, `INFERRED`, and `AMBIGUOUS`.

Route freshness is tracked separately from graph rows. A node or edge can still exist while a route is stale, so use route status when current behavior or recently changed files matter.

Stale graph state is soft-closed. Treat stale rows as historical evidence unless the user explicitly asks about history; prefer active rows for current code questions.

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

## Progressive References

- `references/mcp-tools.md` - MCP Tools And Resources
- `references/rust-extraction.md` - Rust Extraction
- `references/csharp-extraction.md` - CSharp Extraction
- `references/local-testbeds.md` - Local Testbeds
- `references/agent-handoffs.md` - Agent Handoffs
- `references/troubleshooting.md` - Troubleshooting
