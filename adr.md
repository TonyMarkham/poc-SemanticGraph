# ADR: Durable SQLite Storage for a Rust/C# Semantic Code Graph

Status: Proposed

Date: 2026-06-13

## Context

This repository includes Graphify as a submodule at `submodules/graphify`
(`813db192520971a4e880fab7308b9cca886a0333` at the time of audit). Graphify is
useful as a reference implementation for graph shape, confidence tagging,
source provenance, and downstream graph analysis. It is not a direct persistence
model to copy.

The target use case here is narrower than Graphify's full multimodal corpus
support:

- Coding projects are limited to Rust and C#.
- Rust semantic facts should come from `rust-analyzer`.
- C# semantic facts should come from `csharp-language-server`.
- The graph should be durable, incrementally refreshable, and auditable.

## Graphify Findings

Graphify builds a graph in three passes. Code structure is extracted locally
with tree-sitter; code-only corpora skip the LLM semantic extraction path.
Documents, papers, images, and transcripts are the primary inputs for LLM
semantic extraction. See:

- `submodules/graphify/docs/how-it-works.md:7-16`

Graphify's persisted graph format is NetworkX node-link JSON. Nodes have stable
IDs, labels, file types, and source files. Edges have source, target, relation,
confidence, optional confidence score, and source file. Hyperedges are stored as
graph metadata. See:

- `submodules/graphify/docs/how-it-works.md:83-98`

Graphify uses topology for clustering. It does not require embeddings for
community detection; `semantically_similar_to` edges influence the graph
directly. See:

- `submodules/graphify/docs/how-it-works.md:26-30`

Graphify uses three confidence labels:

- `EXTRACTED`: directly observed in source.
- `INFERRED`: model or heuristic inference.
- `AMBIGUOUS`: uncertain and worth review.

See:

- `submodules/graphify/docs/how-it-works.md:34-49`
- `submodules/graphify/graphify/skills/codex/references/extraction-spec.md:12-28`

Graphify's build path defaults to an undirected `nx.Graph` for backward
compatibility, then stores `_src` and `_tgt` on edges to recover true direction
when exporting. This is a sign that a durable store should model direction as a
first-class property instead of relying on undirected graph storage. See:

- `submodules/graphify/graphify/build.py:107-114`
- `submodules/graphify/graphify/build.py:260-279`
- `submodules/graphify/graphify/export.py:510-531`

Graphify's node merge behavior is overwrite-oriented. Same-ID nodes overwrite
previous attributes depending on extraction order, and deduplication rewrites
edges before graph construction. This works for a generated artifact, but a
durable database should preserve raw evidence separately from canonical graph
state. See:

- `submodules/graphify/graphify/build.py:3-21`
- `submodules/graphify/graphify/build.py:303-321`

Graphify already has Rust and C# extractors, but they are syntax-first:

- Rust is tree-sitter based and emits functions, structs, enums, traits, impl
  methods, use declarations, references, implementations, and calls.
  See `submodules/graphify/graphify/extract.py:6300-6574`.
- C# delegates to the generic tree-sitter extractor.
  See `submodules/graphify/graphify/extract.py:4049-4051`.
- C# inheritance/interface relationships are inferred from base lists.
  See `submodules/graphify/graphify/extract.py:2474-2520`.

For Rust and C# workspaces, LSP-backed extraction should replace most of this
bespoke interrogation path because the language servers can resolve symbols
semantically instead of matching names syntactically.

## Language Server Findings

`rust-analyzer` exposes the standard capabilities needed for a semantic graph:
definition, type definition, implementation, references, document symbols,
workspace symbols, call hierarchy, and semantic tokens. See:

- `submodules/rust-analyzer/crates/rust-analyzer/src/lsp/capabilities.rs:68-145`
- `submodules/rust-analyzer/crates/rust-analyzer/src/main_loop.rs:1336-1362`

`rust-analyzer` implements outgoing call hierarchy by resolving callable
expressions and method calls through semantic analysis. See:

- `submodules/rust-analyzer/crates/rust-analyzer/src/handlers/request.rs:1852-1942`
- `submodules/rust-analyzer/crates/ide/src/call_hierarchy.rs:101-155`

`csharp-language-server` exposes definition, references, document symbols,
workspace symbols, implementation, type definition, semantic tokens, call
hierarchy, and type hierarchy. See:

- `submodules/csharp-language-server/src/CSharpLanguageServer/Lsp/Server.fs:76-103`
- `submodules/csharp-language-server/src/CSharpLanguageServer/Lsp/Server.fs:187-210`

C# references are backed by Roslyn `SymbolFinder.FindReferencesAsync`, including
regular and source-generated documents. See:

- `submodules/csharp-language-server/src/CSharpLanguageServer/Handlers/References.fs:45-92`

C# call hierarchy currently has a limitation: incoming calls are implemented
with Roslyn `FindCallersAsync`, but outgoing calls return `None`. See:

- `submodules/csharp-language-server/src/CSharpLanguageServer/Handlers/CallHierarchy.fs:82-142`

This means the schema should support `calls` edges, but the ingestion strategy
for C# must not assume `callHierarchy/outgoingCalls` is available from
`csharp-language-server`.

## Decision

Use SQLite as the durable graph store with:

1. Canonical `nodes` and `edges`.
2. Separate `occurrences` and `edge_evidence` tables for source-level proof.
3. Extraction run tracking for incremental refresh and soft deletion.
4. Directed edges by default.
5. Graphify-compatible confidence labels.
6. Optional hyperedges and derived analysis tables.
7. JSON columns only for provider-specific or rarely queried metadata.
8. A version-control-friendly plain-text snapshot using Frictionless Data
   Package conventions: `datapackage.json` plus deterministic CSV files.

Do not persist NetworkX node-link JSON as the source of truth. It can still be
exported from SQLite for visualization or compatibility.

## Base Reasoning

Graphify's JSON artifact is compact and useful, but it is an output artifact.
It intentionally merges, deduplicates, normalizes, and sometimes drops
intermediate information. A durable database should preserve the observations
that justify a graph edge:

- Which LSP request produced it.
- Which source range proves it.
- Which extraction run observed it.
- Whether the edge was directly observed, inferred, or ambiguous.
- Whether a later run failed to observe it again.

For Rust and C#, the best source of truth is not text syntax alone. LSP servers
understand project configuration, packages/crates, generated code, type
resolution, implementations, references, and call relationships. Those facts
should become `EXTRACTED` evidence. Graphify-style inferred semantic edges
should be optional enrichment, not the core code graph.

## Recommended SQLite Schema

```sql
PRAGMA foreign_keys = ON;

CREATE TABLE workspaces (
  id INTEGER PRIMARY KEY,
  root_uri TEXT NOT NULL UNIQUE,
  kind TEXT NOT NULL CHECK (kind IN ('rust', 'csharp', 'mixed')),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE extraction_runs (
  id INTEGER PRIMARY KEY,
  workspace_id INTEGER NOT NULL REFERENCES workspaces(id),
  provider TEXT NOT NULL,
  provider_version TEXT,
  git_commit TEXT,
  started_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  finished_at TEXT,
  status TEXT NOT NULL DEFAULT 'running'
    CHECK (status IN ('running', 'complete', 'failed')),
  properties_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(properties_json))
);

CREATE TABLE files (
  id INTEGER PRIMARY KEY,
  workspace_id INTEGER NOT NULL REFERENCES workspaces(id),
  uri TEXT NOT NULL,
  path TEXT NOT NULL,
  language TEXT NOT NULL CHECK (language IN ('rust', 'csharp', 'markdown', 'other')),
  content_hash TEXT,
  last_seen_run_id INTEGER REFERENCES extraction_runs(id),
  properties_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(properties_json)),
  UNIQUE (workspace_id, uri)
);

CREATE TABLE nodes (
  id TEXT PRIMARY KEY,
  workspace_id INTEGER NOT NULL REFERENCES workspaces(id),
  language TEXT NOT NULL CHECK (language IN ('rust', 'csharp', 'markdown', 'other')),
  kind TEXT NOT NULL,
  name TEXT NOT NULL,
  qualified_name TEXT,
  display_name TEXT,
  symbol_key TEXT NOT NULL,
  file_id INTEGER REFERENCES files(id),
  start_line INTEGER,
  start_col INTEGER,
  end_line INTEGER,
  end_col INTEGER,
  selection_start_line INTEGER,
  selection_start_col INTEGER,
  container_node_id TEXT REFERENCES nodes(id),
  properties_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(properties_json)),
  first_seen_run_id INTEGER REFERENCES extraction_runs(id),
  last_seen_run_id INTEGER REFERENCES extraction_runs(id),
  valid_to_run_id INTEGER REFERENCES extraction_runs(id),
  UNIQUE (workspace_id, language, symbol_key)
);

CREATE TABLE edges (
  id TEXT PRIMARY KEY,
  workspace_id INTEGER NOT NULL REFERENCES workspaces(id),
  src_node_id TEXT NOT NULL REFERENCES nodes(id),
  dst_node_id TEXT NOT NULL REFERENCES nodes(id),
  relation TEXT NOT NULL,
  context TEXT,
  confidence TEXT NOT NULL
    CHECK (confidence IN ('EXTRACTED', 'INFERRED', 'AMBIGUOUS')),
  confidence_score REAL NOT NULL,
  weight REAL NOT NULL DEFAULT 1.0,
  properties_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(properties_json)),
  first_seen_run_id INTEGER REFERENCES extraction_runs(id),
  last_seen_run_id INTEGER REFERENCES extraction_runs(id),
  valid_to_run_id INTEGER REFERENCES extraction_runs(id),
  UNIQUE (workspace_id, src_node_id, dst_node_id, relation, context)
);

CREATE TABLE edge_evidence (
  id INTEGER PRIMARY KEY,
  edge_id TEXT NOT NULL REFERENCES edges(id),
  run_id INTEGER NOT NULL REFERENCES extraction_runs(id),
  provider TEXT NOT NULL,
  lsp_method TEXT,
  file_id INTEGER REFERENCES files(id),
  start_line INTEGER,
  start_col INTEGER,
  end_line INTEGER,
  end_col INTEGER,
  raw_json TEXT CHECK (raw_json IS NULL OR json_valid(raw_json))
);

CREATE TABLE occurrences (
  id INTEGER PRIMARY KEY,
  node_id TEXT NOT NULL REFERENCES nodes(id),
  run_id INTEGER NOT NULL REFERENCES extraction_runs(id),
  file_id INTEGER NOT NULL REFERENCES files(id),
  role TEXT NOT NULL CHECK (
    role IN (
      'definition',
      'reference',
      'call',
      'import',
      'implementation',
      'override'
    )
  ),
  start_line INTEGER NOT NULL,
  start_col INTEGER NOT NULL,
  end_line INTEGER NOT NULL,
  end_col INTEGER NOT NULL,
  enclosing_node_id TEXT REFERENCES nodes(id),
  raw_json TEXT CHECK (raw_json IS NULL OR json_valid(raw_json))
);

CREATE TABLE hyperedges (
  id TEXT PRIMARY KEY,
  workspace_id INTEGER NOT NULL REFERENCES workspaces(id),
  label TEXT NOT NULL,
  relation TEXT NOT NULL,
  confidence TEXT NOT NULL
    CHECK (confidence IN ('EXTRACTED', 'INFERRED', 'AMBIGUOUS')),
  confidence_score REAL NOT NULL,
  properties_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(properties_json))
);

CREATE TABLE hyperedge_members (
  hyperedge_id TEXT NOT NULL REFERENCES hyperedges(id),
  node_id TEXT NOT NULL REFERENCES nodes(id),
  ordinal INTEGER NOT NULL,
  PRIMARY KEY (hyperedge_id, node_id)
);

CREATE TABLE graph_snapshots (
  id INTEGER PRIMARY KEY,
  workspace_id INTEGER NOT NULL REFERENCES workspaces(id),
  run_id INTEGER NOT NULL REFERENCES extraction_runs(id),
  kind TEXT NOT NULL CHECK (kind IN ('community', 'centrality', 'export')),
  algorithm TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  properties_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(properties_json))
);

CREATE TABLE node_metrics (
  snapshot_id INTEGER NOT NULL REFERENCES graph_snapshots(id),
  node_id TEXT NOT NULL REFERENCES nodes(id),
  community_id INTEGER,
  degree INTEGER,
  betweenness REAL,
  properties_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(properties_json)),
  PRIMARY KEY (snapshot_id, node_id)
);

CREATE INDEX idx_files_workspace_path
  ON files(workspace_id, path);

CREATE INDEX idx_nodes_workspace_qname
  ON nodes(workspace_id, qualified_name);

CREATE INDEX idx_nodes_file
  ON nodes(file_id);

CREATE INDEX idx_edges_src
  ON edges(src_node_id);

CREATE INDEX idx_edges_dst
  ON edges(dst_node_id);

CREATE INDEX idx_edges_relation
  ON edges(workspace_id, relation);

CREATE INDEX idx_occurrences_node_role
  ON occurrences(node_id, role);

CREATE INDEX idx_occurrences_file
  ON occurrences(file_id);

CREATE INDEX idx_edge_evidence_edge
  ON edge_evidence(edge_id);
```

Recommended FTS table:

```sql
CREATE VIRTUAL TABLE node_search USING fts5(
  node_id UNINDEXED,
  name,
  qualified_name,
  display_name,
  file_path
);
```

Keep this table synchronized from application code or triggers.

## Version-Control Snapshot

Use a Frictionless Data Package style export as the commit-friendly plain-text
representation of the graph database:

```text
graph-export/
  datapackage.json
  schema.sql
  tables/
    workspaces.csv
    extraction_runs.csv
    files.csv
    nodes.csv
    edges.csv
    edge_evidence.csv
    occurrences.csv
    hyperedges.csv
    hyperedge_members.csv
    graph_snapshots.csv
    node_metrics.csv
```

This is a better fit than a single SQL dump or a single JSON graph file:

- CSV is plain text and diffable.
- One file per table avoids Git LFS for normal-sized changes.
- Deterministic ordering by primary key keeps diffs stable.
- `datapackage.json` can describe table schemas, primary keys, foreign keys,
  row counts, table hashes, schema version, export version, and source commit.
- `schema.sql` preserves SQLite-specific DDL such as indexes, constraints, FTS
  tables, and triggers.

Treat `graph-export/` as a materialized graph snapshot, not just a backup.
SQLite remains the runtime/query format; the Data Package is the versionable
transport format.

Recommended export rules:

- Sort each CSV by primary key.
- Use stable column order matching `schema.sql`.
- Use UTF-8 and RFC 4180-compatible CSV quoting.
- Exclude volatile SQLite internals, WAL files, FTS backing tables, and local
  cache tables.
- Include enough run metadata to preserve incremental state, especially
  `first_seen_run_id`, `last_seen_run_id`, and `valid_to_run_id`.
- Keep derived metrics in separate tables so importers can choose to trust them
  or recompute them.

## SQLite Preheat Workflow

Use the Data Package snapshot to preheat SQLite before running incremental
analysis:

1. Checkout the repository.
2. Create a fresh `graph.db` from `graph-export/schema.sql`.
3. Bulk load `graph-export/tables/*.csv`.
4. Rebuild indexes and FTS tables.
5. Start the analysis/query stack against the preheated SQLite database.
6. Detect changed files since the snapshot's source commit.
7. Ask `rust-analyzer` / `csharp-language-server` only for changed files and
   affected adjacent symbols.
8. Update canonical nodes, edges, occurrences, and evidence.
9. Recompute impacted graph projections where possible.
10. Re-export deterministic CSV when the refreshed graph should be committed.

For import speed, use bulk-load settings during the preheat step:

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = OFF;
-- bulk import tables
PRAGMA foreign_keys = ON;
-- create/rebuild indexes and FTS last
```

Foreign keys should be validated after import. If the importer disables foreign
keys for speed, it should run `PRAGMA foreign_key_check;` before treating the
database as ready.

## ID Strategy

Use deterministic IDs:

- `nodes.id = sha256(workspace_id || language || symbol_key)`
- `edges.id = sha256(workspace_id || src_node_id || dst_node_id || relation || context)`
- `hyperedges.id = sha256(workspace_id || relation || sorted_member_ids || label)`

For Rust, prefer a symbol key derived from `rust-analyzer` navigation identity:
definition URI, selection range, symbol kind, and qualified/container name.

For C#, prefer a Roslyn symbol key if exposed by the ingestion path. If only LSP
locations are available, use definition URI, selection range, symbol kind, and
fully qualified display name.

Do not use Graphify's `{stem}_{entity}` ID format as the durable primary key.
It is useful for JSON compatibility, but it is vulnerable to refactors,
overloads, partial classes, generated code, and same-name symbols in different
containers.

## Edge Model

Recommended core relations:

- `contains`: file/module/type contains child symbol.
- `defines`: file defines symbol.
- `imports`: file or module imports namespace/module/crate item.
- `references`: symbol references another symbol.
- `calls`: callable invokes another callable.
- `implements`: type implements trait/interface.
- `inherits`: type derives from type or trait/interface extends another.
- `type_of`: expression/member has resolved type.
- `overrides`: member overrides or implements inherited member.
- `semantically_similar_to`: optional Graphify-style inferred relation.

Confidence rules:

- LSP-resolved facts: `EXTRACTED`, `1.0`.
- Name-only fallback: `INFERRED`, usually `0.65` or `0.75`.
- Ambiguous or conflicting resolution: `AMBIGUOUS`, `0.1` to `0.3`.
- LLM/document-derived conceptual edges: follow Graphify's confidence rubric.

## Ingestion Strategy

Rust:

1. Start `rust-analyzer` for the workspace.
2. Use `textDocument/documentSymbol` to seed file-local nodes and containment.
3. Use `workspace/symbol` for workspace-wide symbol discovery where useful.
4. Use `textDocument/definition`, `textDocument/typeDefinition`,
   `textDocument/implementation`, and `textDocument/references` to resolve
   relationships.
5. Use `textDocument/prepareCallHierarchy` plus incoming/outgoing call hierarchy
   to populate `calls`.
6. Store every source range in `occurrences` and every relationship proof in
   `edge_evidence`.

C#:

1. Start `csharp-language-server` with the intended solution.
2. Use `textDocument/documentSymbol` and `workspace/symbol` for nodes.
3. Use `textDocument/definition`, `textDocument/typeDefinition`,
   `textDocument/implementation`, and `textDocument/references` for resolved
   relationships.
4. Use `typeHierarchy/supertypes` and `typeHierarchy/subtypes` for inheritance
   and implementation relationships.
5. Use incoming call hierarchy where available.
6. For outgoing calls, do not assume `callHierarchy/outgoingCalls` works in the
   current `csharp-language-server`. Use one of:
   - inverted incoming calls from all callable symbols,
   - references plus enclosing-symbol attribution,
   - a Roslyn-side extractor,
   - or mark heuristic call edges as `INFERRED`.

## Consequences

Benefits:

- Preserves auditability and source evidence.
- Supports incremental updates and soft deletion.
- Keeps Graphify-compatible confidence semantics.
- Avoids undirected-edge direction loss.
- Lets Rust/C# language servers handle semantic resolution.
- Can export to Graphify-compatible node-link JSON later.

Costs:

- More tables than a simple nodes/edges store.
- Requires an ingestion layer that can drive LSP requests.
- C# outgoing calls require extra handling because `csharp-language-server`
  currently does not implement outgoing call hierarchy.

## Export Compatibility

A Graphify-compatible export can be generated from SQLite:

- `nodes.id` -> JSON node `id`
- `nodes.display_name || nodes.name` -> `label`
- `files.path` -> `source_file`
- source range columns -> `source_location`
- `edges.src_node_id` -> `source`
- `edges.dst_node_id` -> `target`
- `edges.relation`, `confidence`, `confidence_score`, `weight`, `context`
- `hyperedges` and `hyperedge_members` -> top-level `hyperedges`
- latest `node_metrics.community_id` -> node `community`

The SQLite database remains the source of truth; Graphify JSON is an export
format.
