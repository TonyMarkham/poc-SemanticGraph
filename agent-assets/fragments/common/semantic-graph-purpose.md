## When To Use This Skill

Use this skill when the user's task is about code relationships, references, call graphs, source provenance, evidence, route freshness, stale graph facts, SemanticGraph MCP query work, or planning and validating SemanticGraph refreshes.

Do not claim every task must query the graph first. For unrelated formatting, copy edits, build failures, or ordinary local code changes, use the graph only when relationship, provenance, freshness, or graph-refresh facts matter.

## SemanticGraph Facts

SemanticGraph stores canonical graph data in SQLite `nodes` and `edges`, with source proof in `occurrences` and `edge_evidence`. Directed edges are first-class. Confidence values are `EXTRACTED`, `INFERRED`, and `AMBIGUOUS`.

Route freshness is tracked separately from graph rows. A node or edge can still exist while a route is stale, so use route status when current behavior or recently changed files matter.

Stale graph state is soft-closed. Treat stale rows as historical evidence unless the user explicitly asks about history; prefer active rows for current code questions.
