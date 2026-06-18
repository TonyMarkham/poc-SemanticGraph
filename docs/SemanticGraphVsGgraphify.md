# SemanticGraph vs Graphify: A Brutally Honest Review

## Framing

SemanticGraph is not primarily a graph visualizer. The visualizer is a useful
debugging and inspection surface for humans, but it is not the product center.

SemanticGraph is primarily an LLM efficiency tool. Its job is to let an agent
answer codebase questions and plan code changes with fewer raw file reads, less
context flooding, better currentness checks, and stronger source evidence than a
grep-first workflow.

Viewed through that lens, Graphify is the right reference, but not because
SemanticGraph should become Graphify. Graphify is a broad, polished assistant
product. SemanticGraph is a narrower, deeper semantic infrastructure layer. The
best outcome is not feature parity. The best outcome is to steal Graphify's
agent ergonomics while keeping SemanticGraph's durable, route-aware, evidence
preserving core.

Local reference points:

- SemanticGraph schema: `crates/semantic-graph-store/migrations/01_create_graph_store.sql`
- SemanticGraph route freshness: `crates/semantic-graph-store/migrations/02_route_freshness.sql`
- SemanticGraph MCP tools: `crates/semantic-graph-mcp-server/src/tools/tool_registry.rs`
- SemanticGraph extractor CLI: `crates/semantic-graph-extract/src/cli/command.rs`
- Graphify behavior docs: `submodules/graphify/docs/how-it-works.md`
- Graphify builder/export/query code: `submodules/graphify/graphify/build.py`,
  `submodules/graphify/graphify/export.py`, `submodules/graphify/graphify/serve.py`

## Bottom Line

SemanticGraph has the better long-term foundation for code-aware LLM work:
SQLite durability, first-class directed edges, active/stale state, route
freshness, raw occurrence/evidence tables, and language-server-backed Rust/C#
facts. Those are the right primitives for trustworthy agent context.

Graphify has the better product. It is dramatically easier to explain, easier
to run, easier to query, easier to export, and more obviously useful to an AI
assistant immediately after installation.

That gap matters. An LLM efficiency tool that exposes excellent primitives but
requires the agent to manually compose search, details, neighbors, route status,
and evidence lookups will underperform a less precise tool that gives the agent
one obvious command returning a compact, relevant subgraph.

SemanticGraph's current risk is not that its storage is weak. The risk is that
the system is too honest and too low-level to be efficient for the caller that
matters most: the agent.

## Where SemanticGraph Is Stronger

### 1. Durable Truth Model

SemanticGraph's graph is stored in SQLite with canonical `nodes` and `edges`
and separate proof in `occurrences` and `edge_evidence`. That is the correct
shape for an LLM efficiency system because the agent can ask not only "what is
connected?" but "why do we believe this?" and "is this fact still fresh?"

Graphify's `graph.json` is portable and friendly, but it is still fundamentally
a serialized graph artifact. It is good for movement across tools. It is not as
good as a mutable, queryable, evidence-preserving database.

Verdict: SemanticGraph is architecturally stronger.

### 2. Freshness And Staleness

SemanticGraph tracks route freshness separately from graph facts. A symbol,
edge, occurrence, or evidence row can be preserved while active state is
soft-closed. This is exactly what an agent needs when it is deciding whether a
fact is current enough to trust.

Graphify has content hashes, cache, incremental scan, and deleted-file pruning.
That is practical and product-friendly. But it does not provide the same
fine-grained route-level freshness contract.

Verdict: SemanticGraph is better for serious incremental code intelligence.

### 3. Language-Server Semantics

SemanticGraph's Rust path uses the checked-in `rust-analyzer` facade, and the C#
path uses `csharp-ls-lib`. For Rust/C#, this is much more valuable than broad
tree-sitter extraction. Tree-sitter can identify structure. A language server
can resolve references and calls with actual compiler/language knowledge.

Graphify covers many more languages, but the cost is shallower semantics.
Graphify is excellent at broad orientation. SemanticGraph is better suited to
high-confidence code navigation in the languages it supports.

Verdict: SemanticGraph wins narrowly but deeply; Graphify wins broadly but
shallowly.

### 4. Agent-Facing Structured APIs

SemanticGraph already has the right primitive MCP tools:

- `graph_stats`
- `graph_search_nodes`
- `graph_node_details`
- `graph_edge_details`
- `graph_projection`
- `graph_neighbors`
- `graph_shortest_path`
- `graph_file_summary`
- `graph_route_status`
- `fts_search`

This is a strong base. It exposes the database as structured context rather
than prose. That makes it easier for agents to reason precisely.

Verdict: the primitive API is good. The workflow API is not good enough yet.

### 5. Graph Search Plus Raw-Text FTS

SemanticGraph can search the extracted graph and indexed raw file text. That
matters for LLM efficiency. An agent can use the graph to orient around symbols
and relations, then use `fts_search` to find targeted raw-text evidence before
deciding whether a whole file read is worth the context cost.

Graphify appears to have graph search, not full-text search in this sense. Its
query path searches extracted graph data and traverses relationships in
`graph.json`. That is useful, but it is not the same as a Tantivy-backed index
over repository file contents with snippet hydration from stored text.

This is a real SemanticGraph advantage. It gives the agent a middle step between
"trust the graph" and "read the file." That middle step is exactly where many
tokens can be saved.

Verdict: SemanticGraph has the better search stack for code-agent efficiency.

## Where Graphify Is Stronger

### 1. It Understands The Assistant Workflow Better

Graphify is aggressively assistant-first. It installs reminders, hooks, skills,
and commands that tell the agent to query the graph before reading files. It
does not wait for the agent to remember the graph exists.

SemanticGraph has Codex asset installation work, but the product story is still
less forceful. The system should make the efficient path the default path. Today
it still feels like the agent has to be well-trained to use SemanticGraph
correctly.

Brutal version: Graphify is annoying in exactly the right way. SemanticGraph is
polite in a way that risks being ignored.

### 2. It Has LLM-Shaped Query Outputs

Graphify's `query_graph`, `get_node`, `get_neighbors`, `god_nodes`, and
`shortest_path` surfaces are not perfect, but they are designed to return
readable, compact context to an assistant.

SemanticGraph's tools are more correct but more atomic. An agent often has to:

1. Search for nodes.
2. Pick a node.
3. Fetch details.
4. Fetch neighbors.
5. Fetch route status.
6. Fetch evidence.
7. Possibly search file text.
8. Synthesize the answer.

That is not terrible for a careful agent. It is bad for efficiency. The tool
should compose that common path.

### 3. It Produces Orientation Artifacts

Graphify's `GRAPH_REPORT.md` is not just a human nicety. It is cheap context for
an LLM. God nodes, surprising connections, community summaries, and suggested
questions create a fast orientation layer.

SemanticGraph has the data to generate something better, but it does not yet
produce a comparable compact repository briefing from SQLite.

Brutal version: SemanticGraph has stronger facts but weaker memory.

### 4. It Has A Single Product Command

Graphify has a clean mental model:

```sh
graphify .
graphify query "..."
graphify export ...
```

SemanticGraph currently exposes multiple binaries and surfaces:

- `semantic-graph-extract`
- `semantic-graph-store`
- `semantic-graph-mcp-server`
- `semantic-graph-visualizer-server`
- `semantic-graph`

That separation is clean architecturally, but it is not clean as a product. For
an LLM efficiency tool, every extra setup step is a chance for the agent or
human to stop using it.

### 5. It Has Better Broad-Corpus Hygiene

Graphify has explicit sensitive-file skipping, `.graphifyignore`/`.gitignore`
behavior, broad file classification, size caps for risky formats, and many
practical scars from scanning arbitrary folders.

SemanticGraph FTS has configurable ignores and a max indexed file size, but it
does not appear to have Graphify's level of built-in secret-file heuristics.
That is acceptable for a controlled prototype. It is not acceptable for a
general agent-installed tool that may index arbitrary repositories.

## What SemanticGraph Should Not Copy

### Do Not Copy Graphify's Durable Storage Model

NetworkX node-link JSON is great for export. It should not replace SQLite.

SemanticGraph's database is the better source of truth. Use JSON/GraphML/CSV as
export formats, not as the canonical graph.

### Do Not Chase Multimodal Breadth Yet

Graphify handles docs, PDFs, images, videos, Office files, Google Workspace,
Postgres, Terraform, and a long tail of languages. That breadth is impressive,
but it would dilute SemanticGraph's current advantage.

SemanticGraph should first become the best Rust/C# codebase context system for
LLMs. Multimodal ingestion can come later, and it should not compromise
evidence quality.

### Do Not Treat The Visualizer As The Main Competition

Graphify's browser export is useful, but SemanticGraph does not need to beat it
there first. The Blazor visualizer should help inspect, debug, and trust the
graph. It should not absorb the core roadmap.

The primary UI is the MCP tool surface.

## What SemanticGraph Should Steal

### 1. A High-Level `query_graph` Tool

Add an MCP tool that takes a natural language or keyword query and returns a
compact, evidence-backed subgraph:

- matching nodes;
- top active neighbors;
- relevant incoming/outgoing edges;
- short occurrence/evidence snippets;
- route freshness warnings;
- file text hits when useful;
- an explicit "confidence/currentness" section.

This should be the default tool an agent reaches for before raw file reads.

### 2. An `explain_symbol` Tool

The existing `graph_node_details` is precise but too raw. Add a higher-level
tool that summarizes:

- what the symbol is;
- where it is defined;
- who contains it;
- what it calls;
- what calls it;
- what references it;
- what evidence supports those claims;
- whether any related routes are stale.

This is where SemanticGraph can beat Graphify: concise prose backed by exact
SQLite evidence.

### 3. A `summarize_file` Tool

`graph_file_summary` is already close. Make an LLM-shaped version:

- file purpose inferred from symbols and relations;
- top definitions;
- calls/references crossing file boundaries;
- route freshness;
- relevant FTS snippets;
- "read this file now only if..." guidance.

The goal is not to hide source files forever. The goal is to delay expensive raw
reads until they are actually justified.

### 4. A `repo_brief` Artifact

Generate a checked or local Markdown briefing from SQLite:

- top modules/files by graph centrality;
- high-degree symbols;
- changed or stale routes;
- ambiguous edges;
- important call chains;
- recently refreshed workspaces;
- suggested agent queries.

This should be SemanticGraph's equivalent of Graphify's `GRAPH_REPORT.md`, but
more factual and less narrative.

### 5. Better Install-Time Defaults

The installed Codex assets should be more opinionated:

- use SemanticGraph MCP first for codebase questions;
- use `fts_search` before shell text search;
- use route status when currentness matters;
- use raw file reads only after graph orientation;
- refresh only when explicitly allowed.

This repo already has the right philosophy in `AGENTS.md` and the SemanticGraph
skill. It needs to become the installed default for users, not just a local repo
convention.

### 6. Export Formats For Interop

SemanticGraph should export:

- deterministic CSV/Data Package snapshots;
- NetworkX node-link JSON;
- GraphML;
- maybe Cypher later.

This is not because exports are the core. It is because agents and external
tools benefit from cheap, portable context artifacts.

## Priority Recommendations

### P0: Build Agent-Efficient Composite Tools

Do this before more UI work.

The highest-leverage tools are:

- `query_graph`
- `explain_symbol`
- `summarize_file`
- `affected_by_file`
- `repo_brief`

These tools should reduce multi-call agent workflows into one call that returns
compact, source-grounded context.

### P1: Generate A SQLite-Backed Briefing

Create a report command that writes something like:

```text
semantic-graph report --output .refactor-radar/SEMANTIC_GRAPH_REPORT.md
```

It should not pretend to be an LLM-written architecture essay. It should be a
factual briefing for agents.

### P1: Promote Route Freshness In Every Answer

Route freshness is one of SemanticGraph's strongest differentiators, but it is
too easy for a caller to ignore. High-level tools should surface freshness
status automatically.

If an agent answers from stale facts without noticing, the system has failed.

### P2: Add Safer FTS Discovery Defaults

Before generalizing beyond this repo, FTS needs stronger default exclusions for
secret-looking files and directories. Configurable ignores are not enough when
the tool is supposed to protect users by default.

### P2: Expose More Query Power In The Visualizer

The visualizer should eventually expose neighbors, shortest path, file summary,
route status, and FTS. But this is secondary. Those features matter first as MCP
tools.

## Honest Scorecard

| Area | SemanticGraph | Graphify | Winner |
| --- | --- | --- | --- |
| Durable source of truth | SQLite with provenance | NetworkX JSON artifact | SemanticGraph |
| Freshness/currentness | Route-scoped status and stale closing | Content cache and pruning | SemanticGraph |
| Rust/C# semantic precision | Language-server backed | Tree-sitter/heuristic | SemanticGraph |
| Raw file full-text search | Tantivy plus SQLite snippets | No comparable FTS found | SemanticGraph |
| Breadth of inputs | Narrow | Very broad | Graphify |
| Assistant ergonomics | Good primitives, weak composites | Strong workflow | Graphify |
| LLM-ready summaries | Underbuilt | Stronger | Graphify |
| Human visualization/export | Prototype UI | Many exports | Graphify |
| Safety for arbitrary corpora | Partial | More mature | Graphify |
| Long-term code intelligence foundation | Strong | Mixed | SemanticGraph |

## Final Verdict

SemanticGraph should not try to become Graphify. That would waste its best
advantage.

SemanticGraph should become the evidence-grade, route-fresh, language-server
backed memory layer that Graphify cannot easily be. But it must learn from
Graphify's product instincts: agents need obvious, compact, high-level context
tools, not just accurate primitives.

The brutal truth is that SemanticGraph's backend is ahead of its user story.
The data model is serious. The extraction pipeline is serious. The MCP primitive
set is promising. But as an LLM efficiency tool, the system is not finished
until an agent can reliably save context and time without being an expert in
SemanticGraph's internals.

The next milestone should not be a prettier graph. It should be fewer agent
tool calls, fewer raw file reads, and better answers with explicit evidence and
freshness.
