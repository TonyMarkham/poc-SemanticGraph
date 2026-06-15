# VS-30: Semantic Graph MCP Server And Codex Skill

Status: Draft implementation plan
Date: 2026-06-14

## Goal

Add a local MCP server and a Codex skill so agents use this repo's semantic
graph as the default tool for live-code symbol, reference, call, and audit
questions.

The intended end state is:

- the semantic graph database is exposed through named MCP tools;
- common questions do not require ad hoc SQL or `rg` as the first move;
- tool responses include source evidence and freshness metadata;
- Codex has a durable skill that routes semantic questions to the MCP tools;
- `rg` remains the default for raw text, comments, strings, docs, and files not
  covered by the semantic graph.

## Motivation

The project now has real semantic value: Rust document symbols, references,
calls, evidence rows, route freshness, and an all-in-one extraction command.
That makes questions like "where is `std::result::Result` used in live code?"
different from plain text search.

Text search can find literal tokens. The semantic graph can answer resolved
questions from the model:

- which symbols exist;
- which occurrences reference a symbol;
- which symbols call which symbols;
- which routes observed the facts;
- whether a fact is current or stale;
- which source ranges and provider evidence support the answer.

VS-30 turns that value into a stable agent-facing interface.

## Non-Goals

- Do not replace `rg` for full-text search.
- Do not serialize full source text into SQLite.
- Do not add new Rust extraction routes in VS-30.
- Do not change the VS-10 or VS-20 persistence semantics.
- Do not expose unrestricted write access through MCP.
- Do not run extraction automatically from query tools unless the user
  explicitly asks for a refresh or approves that behavior.
- Do not shell out to an installed `rust-analyzer` CLI.

## Current Evidence

The current repo already has the core data needed for an MCP query layer:

- `nodes` store current and stale symbols with kind, name, qualified name,
  file, ranges, symbol key, and `valid_to_run_id`;
- `edges` store current and stale `contains`, `references`, and `calls`
  relations with confidence, context, weight, and route provenance through run
  IDs;
- `occurrences` store definition, reference, and call source ranges;
- `edge_evidence` stores provider, LSP method, file range, and raw JSON proof;
- `extraction_route_status` and `route_observations` store route freshness;
- `node_search` already provides a bounded FTS index over symbol-facing fields;
- `semantic-graph-extract rust-workspace-all` can rebuild a complete Rust
  graph in one command.

The missing layer is a named, evidence-returning query API that Codex can call
directly.

## Dependency On VS-Config

VS-30 should use the database path resolver from VS-Config. The MCP server
should accept `--db` as an override, but its normal local startup path should
come from `.refactor-radar/config.toml`.

The MCP server should not implement its own TOML parsing or config discovery.

## MCP Server Shape

Add a new Rust crate:

```text
crates/semantic-graph-mcp
```

The server should use stdio transport first because it is the simplest local
MCP integration shape for Codex-style tooling. HTTP can be added later if a UI
or long-running service needs it.

Recommended command:

```text
semantic-graph-mcp --config .refactor-radar/config.toml
```

Optional arguments:

```text
--db <path>
--workspace-root <path>
--read-only
--max-results <n>
```

Default behavior should be read-only. The first VS-30 server should not mutate
the graph.

## Tool Design Principles

MCP tools should be narrow and semantic.

Each tool response should include:

- the answer rows;
- file path and source range when available;
- node IDs and edge IDs when relevant;
- route/run/freshness metadata;
- provider/evidence metadata when available;
- a clear flag when the answer is incomplete because the DB is missing, stale,
  or does not contain the required relation family.

Avoid returning giant raw SQL dumps. Prefer compact structured JSON with enough
proof for the agent to cite or inspect.

## Initial MCP Tools

### graph_db_status

Purpose: answer whether the configured database exists, opens, has the expected
schema, and contains current route data.

Inputs:

```text
include_routes: bool = true
```

Output:

```text
database_path
exists
schema_ok
workspaces
latest_runs
route_statuses
counts by table
warnings
```

This is the first tool the skill should use before making strong claims from
the DB.

### graph_workspace_stats

Purpose: return high-level semantic graph counts.

Inputs:

```text
workspace_id: optional integer
current_only: bool = true
```

Output:

```text
files
nodes
edges
occurrences
edge_evidence
routes_complete
edges_by_relation
nodes_by_kind
```

### graph_search_symbols

Purpose: find candidate symbols by name, qualified name, path, kind, or simple
query text.

Inputs:

```text
query: string
kind: optional string
language: optional string
current_only: bool = true
limit: integer
```

Output rows:

```text
node_id
kind
name
qualified_name
file_path
range
selection_range
symbol_key
```

Use `node_search` where appropriate, but keep the output semantic.

### graph_find_references

Purpose: return semantic references to one or more resolved target symbols.

Inputs:

```text
target_node_id: optional string
query: optional string
include_definitions: bool = false
current_only: bool = true
limit: integer
```

Behavior:

- if `target_node_id` is provided, query references to that node directly;
- otherwise use `query` to find candidate target nodes and return grouped
  results;
- require `references` route data before claiming completeness.

Output rows:

```text
target_node
source_node_or_file
edge_id
occurrence_range
file_path
confidence
evidence
route_status
```

### graph_find_callers

Purpose: return semantic callers of a target symbol.

Inputs:

```text
target_node_id: optional string
query: optional string
current_only: bool = true
limit: integer
```

Output rows:

```text
caller_node
callee_node
edge_id
callsite_range
file_path
weight
evidence
route_status
```

### graph_find_callees

Purpose: return semantic callees from a caller symbol.

Inputs:

```text
caller_node_id: optional string
query: optional string
current_only: bool = true
limit: integer
```

Output shape should mirror `graph_find_callers`.

### graph_explain_node

Purpose: return a compact evidence-first description of one symbol.

Inputs:

```text
node_id: string
include_edges: bool = true
include_occurrences: bool = true
```

Output:

```text
node fields
definition occurrence
incoming relation counts
outgoing relation counts
sample evidence
freshness
```

### graph_explain_edge

Purpose: return proof for a canonical relation.

Inputs:

```text
edge_id: string
```

Output:

```text
edge fields
source node
destination node
evidence rows
occurrence rows when applicable
route observation metadata
freshness
```

### graph_find_type_pattern_usages

Purpose: support policy audits like "live code using
`std::result::Result` instead of the crate result alias".

Initial VS-30 implementation may be limited to the semantic facts already in
the database. It should not pretend to resolve type aliases until type-use and
import-alias facts exist.

Inputs:

```text
pattern: string
language: string = "rust"
source_set: optional enum later
current_only: bool = true
limit: integer
```

Output rows:

```text
node_id
symbol kind/name
file_path
range
matched_field
matched_value
confidence
limitations
```

For `std::result::Result`, this tool should clearly distinguish:

- confirmed graph facts;
- inferred matches from symbol detail/properties;
- unknown cases that need future resolved type-use extraction.

Do not market this as perfect resolved type search until the missing
type-use/import-alias work exists.

### graph_semantic_sql

Purpose: give expert users and agents a controlled escape hatch for read-only
queries.

Inputs:

```text
sql: string
parameters: optional array
limit: integer
```

Rules:

- allow only `SELECT` and read-only `WITH` statements;
- reject multiple statements;
- reject writes, PRAGMAs that mutate state, extension loading, and attachment;
- apply a hard row limit;
- return column names and JSON rows.

This tool is useful while named tools mature, but agents should prefer named
tools when one exists.

## Skill Shape

Create a repo-specific Codex skill, likely named:

```text
semantic-graph-query
```

Recommended location:

```text
.codex/skills/semantic-graph-query
```

The skill should include:

```text
semantic-graph-query/
  SKILL.md
  agents/openai.yaml
  references/query-routing.md
```

Keep `SKILL.md` short. The body should be procedural, not a copy of the schema.

Required trigger description:

```text
Use when working in poc-SemanticGraph and the user asks about live-code
symbols, references, usages, callers, callees, semantic audits, route
freshness, graph database stats, or whether code uses a resolved API/type.
Prefer the semantic graph MCP tools before text search for symbol-aware
questions.
```

## Skill Routing Rules

The skill should teach Codex this ordering:

1. For symbol/reference/call/usage questions, call `graph_db_status` first.
2. If the database is usable and the relevant route is complete, use the named
   semantic MCP tool.
3. Use `graph_semantic_sql` only when no named tool fits.
4. Use `rg` for comments, docs, strings, raw text, config files, generated
   files, or DB gaps.
5. When falling back to `rg`, say that the semantic graph could not answer the
   specific question and why.
6. Do not claim semantic completeness when the relevant route is absent, stale,
   or unsupported.
7. Never use the installed `rust-analyzer` CLI for Rust semantic extraction.
8. Use `semantic-graph-extract rust-workspace-all` only when the user asks to
   build or refresh the database.

## Skill Examples

The skill should include concise examples like these:

```text
User: where is std::result::Result used in live code?
Action: graph_db_status -> graph_find_type_pattern_usages or graph_semantic_sql.
Use rg only to inspect exact text around returned ranges or to cover DB gaps.
```

```text
User: who calls parse_workspace?
Action: graph_db_status -> graph_search_symbols -> graph_find_callers.
```

```text
User: find every reference to GraphStore::connect
Action: graph_db_status -> graph_search_symbols -> graph_find_references.
```

```text
User: search comments for TODO
Action: use rg first because this is text, not semantic graph structure.
```

## MCP Configuration Plan

Document a local Codex MCP config example after the server exists.

Example shape:

```toml
[mcp_servers.semantic_graph]
command = "cargo"
args = [
  "run",
  "-p",
  "semantic-graph-mcp",
  "--",
  "--config",
  ".refactor-radar/config.toml"
]
```

If running through `cargo run` is too slow for normal use, document a built
binary path instead.

## Config Settings

VS-30 may extend `.refactor-radar/config.toml` with MCP-specific settings:

```toml
[mcp]
read_only = true
max_results = 100
```

Rules:

- `[database].path` remains owned by VS-Config;
- `--db` overrides `[database].path`;
- `--read-only` and `--max-results` override `[mcp]` values;
- MCP settings must not trigger extraction or mutation.

## Database Freshness Policy

The MCP server should not silently rebuild the database. It should report:

- database missing;
- schema missing or migration needed;
- no workspace rows;
- no complete `rust.document_symbols` route;
- no complete `rust.references` route when answering reference questions;
- no complete `rust.calls` route when answering call questions;
- stale rows excluded by `current_only`;
- route diagnostics from the last failed run when relevant.

The skill should tell Codex to ask before refreshing the DB unless the user
explicitly requested a rebuild.

The documented rebuild command remains:

```sh
cargo run -p semantic-graph-extract -- rust-workspace-all \
  --workspace-root .
```

## Implementation Sequence

1. Add `crates/semantic-graph-mcp` to the Cargo workspace.
2. Use VS-Config database path resolution for server startup.
3. Choose and wire the MCP SDK/runtime after checking current local dependency
   fit.
4. Add read-only database connection and schema/status checks.
5. Implement shared query DTOs for nodes, edges, ranges, evidence, and route
   freshness.
6. Implement `graph_db_status`.
7. Implement `graph_workspace_stats`.
8. Implement `graph_search_symbols`.
9. Implement `graph_find_references`.
10. Implement `graph_find_callers` and `graph_find_callees`.
11. Implement `graph_explain_node` and `graph_explain_edge`.
12. Implement the limited `graph_find_type_pattern_usages` audit with explicit
    limitations.
13. Implement guarded `graph_semantic_sql`.
14. Add unit/integration tests against seeded SQLite fixtures.
15. Add the `semantic-graph-query` skill and agent metadata.
16. Document local MCP configuration and common workflows in `README.md`.
17. Run validation and inspect the final diff.

## Tests

MCP server tests:

- returns a clear missing-DB status;
- rejects non-read-only SQL;
- applies result limits;
- returns current symbols only by default;
- includes stale rows only when requested;
- finds symbols through `node_search` and direct node filters;
- returns reference occurrences with source evidence;
- returns callers and callees with callsite evidence;
- explains nodes and edges with relation counts and proof;
- reports missing route coverage instead of returning false certainty.

Skill validation:

- ask a symbol usage question and confirm the skill routes to MCP first;
- ask a raw text/comment question and confirm the skill uses `rg`;
- ask a stale/missing DB question and confirm the skill reports the limitation;
- ask for a DB rebuild and confirm it uses `rust-workspace-all`, not separate
  manual document/reference/call commands;
- ask about Rust semantic extraction and confirm it does not invoke the
  installed `rust-analyzer` CLI.

## Validation Commands

Expected validation after implementation:

```sh
SQLX_OFFLINE=true cargo test -p semantic-graph-store
SQLX_OFFLINE=true cargo test -p semantic-graph-mcp
SQLX_OFFLINE=true cargo test -p semantic-graph-extract
SQLX_OFFLINE=true cargo run -p semantic-graph-smoke-tests
cargo run -p semantic-graph-extract -- rust-workspace-all \
  --workspace-root .
cargo run -p semantic-graph-store -- stats
```

If README command examples or smoke counts change, update `README.md` in the
same change.

Do not present the work as complete while `cargo check`, tests, or MCP startup
emit warnings.

## Acceptance Criteria

VS-30 is complete when all of the following are true:

- Codex can start a local `semantic-graph-mcp` server against the workspace DB;
- MCP startup uses VS-Config database path resolution;
- the server exposes named read-only semantic query tools;
- each named tool returns source evidence and freshness metadata;
- missing or stale DB state is reported explicitly;
- symbol, reference, caller, callee, and evidence inspection questions can be
  answered without ad hoc SQL;
- `graph_semantic_sql` exists as a guarded read-only escape hatch;
- a `semantic-graph-query` skill tells Codex when to use MCP before `rg`;
- the skill preserves `rg` as the correct tool for raw text search;
- the skill tells Codex not to use the installed `rust-analyzer` CLI;
- the README documents how to configure and smoke the MCP server;
- tests and validation commands pass without warnings.

## Risks And Decisions To Preserve

- The graph is semantic, not full-text. Keep raw source search out of SQLite.
- Current type-pattern audits are limited until resolved type-use and import
  alias facts are extracted.
- A generic SQL tool is useful but should not become the normal agent path.
- MCP tools must return proof. Answer-only tools will recreate trust problems.
- Read-only is the right first boundary. Mutation and refresh tools can come
  later after approval semantics are explicit.
