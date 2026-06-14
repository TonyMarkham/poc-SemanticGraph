# SemanticGraph Codebase Analysis

Status: Research note

Date: 2026-06-14

Audited repository path:
`/home/tony/git/poc-SemanticGraph`

Validation run used for current counts:

```sh
SQLX_OFFLINE=true cargo run -p semantic-graph-smoke-tests
```

The validation run completed successfully on 2026-06-14.

## Purpose

This note is a standalone analysis of the current codebase's semantic graph
model, extraction pipeline, durable data lifecycle, query surface, and graph
visualization slice. It is not an ADR and does not change project direction.
Its goal is to preserve a deep, implementation-backed summary so future
semantic modelling work can start from the actual state of the repository.

The short version:

- The repository already has a durable SQLite graph core with canonical
  `nodes` and `edges`, plus separate `occurrences` and `edge_evidence`.
- The only implemented extraction route is Rust document-symbol extraction.
- The implemented graph is currently a containment graph, not yet a call,
  reference, type, implementation, package, or multi-language graph.
- The Rust extraction path is in-process and library-backed, not a runtime
  language-server process path.
- The read-only visualizer is a bounded projection over SQLite, with search,
  selection, node details, edge details, occurrences, and evidence.
- The schema is ahead of the current extractor: it already has run tracking,
  validity columns, confidence, context, raw JSON, and search scaffolding that
  are only partially exercised.

## Evidence Base

Primary files inspected:

- `README.md`
- `Cargo.toml`
- `Justfile`
- `crates/semantic-graph-store/migrations/01_create_graph_store.sql`
- `crates/semantic-graph-store/src/store/graph_store.rs`
- `crates/semantic-graph-store/src/ids.rs`
- `crates/semantic-graph-extract/src/main.rs`
- `crates/semantic-graph-extract/src/document_symbols/paths.rs`
- `crates/semantic-graph-extract/src/document_symbols/mapper.rs`
- `crates/semantic-graph-extract/src/model/*.rs`
- `crates/semantic-graph-extract/src/persist/extraction_persister.rs`
- `crates/semantic-graph-extract/src/providers/rust_analyzer/rust_analyzer_provider.rs`
- `crates/semantic-graph-extract/src/providers/rust_analyzer/rust_document_symbol_mapper.rs`
- `crates/rust-analyzer-lib/src/project/load_workspace.rs`
- `crates/rust-analyzer-lib/src/semantic/document_symbols_for_file.rs`
- `crates/rust-analyzer-lib/src/semantic/document_symbols_for_files.rs`
- `crates/semantic-graph-smoke-tests/src/main.rs`
- `crates/semantic-graph-smoke-tests/src/tests/rust_routes.rs`
- `crates/semantic-graph-visualizer-server/src/query/graph_query_service.rs`
- `crates/semantic-graph-visualizer-server/src/rpc/rpc_handler.rs`
- `crates/semantic-graph-visualizer-server/src/dto/*.rs`
- `apps/SemanticGraph.Visualizer/src/SemanticGraph.Visualizer.Client/Services/GraphClient.cs`
- `apps/SemanticGraph.Visualizer/src/SemanticGraph.Visualizer.Client/Services/GraphDiagramBuilder.cs`
- `apps/SemanticGraph.Visualizer/src/SemanticGraph.Visualizer.Client/Pages/Home.razor`
- `apps/SemanticGraph.Visualizer/src/SemanticGraph.Visualizer.Client/Pages/Home.razor.cs`
- `apps/SemanticGraph.Visualizer/src/SemanticGraph.Visualizer.Client/Models/*.cs`
- `apps/SemanticGraph.Visualizer/src/SemanticGraph.Visualizer.Client/Program.cs`

## Current Measured Shape

The smoke report gives the current implemented graph scale:

| Route | Files | Symbols | Persisted nodes | Persisted edges | Occurrences | Evidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| WIP crate | 4 | 53 | 57 | 53 | 53 | 53 |
| Workspace | 130 | 968 | 1098 | 968 | 968 | 968 |

The node count is larger than the symbol count because every extracted source
file also becomes a `file` node. The edge count matches the symbol count
because every symbol currently receives one containment edge from either its
file node or parent symbol.

Evidence:

- The smoke runner prints package discovery, document-symbol counts,
  persistence counts, workspace discovery counts, and submodule exclusion. See
  `crates/semantic-graph-smoke-tests/src/main.rs:40-65`,
  `crates/semantic-graph-smoke-tests/src/main.rs:102-121`, and
  `crates/semantic-graph-smoke-tests/src/main.rs:173-207`.
- The same route is asserted in tests: WIP discovery is exactly four files,
  workspace discovery excludes `submodules/`, and persisted workspace nodes
  exceed persisted files. See
  `crates/semantic-graph-smoke-tests/src/tests/rust_routes.rs:17-48`,
  `crates/semantic-graph-smoke-tests/src/tests/rust_routes.rs:91-120`, and
  `crates/semantic-graph-smoke-tests/src/tests/rust_routes.rs:158-159`.
- The README documents the same current headline counts and available commands.
  See `README.md:124-137`, `README.md:250-265`, and `README.md:336-345`.

## Core Finding

This codebase currently models a durable semantic graph in three layers:

1. Provider-neutral extraction records.
   The extractor has structs for source files, symbols, relations, batch
   extractions, provider IDs, language IDs, ranges, and raw metadata. See
   `crates/semantic-graph-extract/src/model/extracted_symbol.rs:8`,
   `crates/semantic-graph-extract/src/model/extracted_relation.rs:8`,
   `crates/semantic-graph-extract/src/model/document_symbol_extraction.rs:6`,
   and `crates/semantic-graph-extract/src/model/document_symbol_batch_extraction.rs:6`.

2. Durable canonical graph records.
   SQLite stores workspaces, extraction runs, files, canonical nodes,
   canonical edges, occurrences, edge evidence, and a node-search virtual
   table. See
   `crates/semantic-graph-store/migrations/01_create_graph_store.sql:3-142`.

3. Read-only projection records.
   The visualizer backend maps SQLite rows into JSON-RPC DTOs for bounded graph
   projection, node details, edge details, and node search. See
   `crates/semantic-graph-visualizer-server/src/query/graph_query_service.rs:34-120`
   and
   `crates/semantic-graph-visualizer-server/src/rpc/rpc_handler.rs:45-48`.

The conceptual model is strong, but the implemented semantic scope is narrow.
Today, "semantic graph" means "durable Rust document-symbol containment graph
with source proof." That is a real, useful foundation, but it should not be
mistaken for a full code intelligence graph yet.

## Workspace And Crate Topology

The Rust workspace has six members:

- `crates/rust-analyzer-lib`
- `crates/semantic-graph-extract`
- `crates/semantic-graph-smoke-tests`
- `crates/semantic-graph-store`
- `crates/semantic-graph-visualizer-server`
- `crates/wip`

See `Cargo.toml:2-8`.

Architectural roles:

| Component | Role |
| --- | --- |
| `semantic-graph-store` | SQLite schema, storage API, stats/demo CLI |
| `semantic-graph-extract` | Provider-neutral extraction models, Rust document-symbol route, persistence adapter |
| `rust-analyzer-lib` | In-process Rust project loading and file-structure facade |
| `semantic-graph-smoke-tests` | Cross-crate smoke report for discovery, extraction, and persistence |
| `semantic-graph-visualizer-server` | Local read-only JSON-RPC backend over SQLite |
| `SemanticGraph.Visualizer.Client` | Blazor WebAssembly graph viewport and inspector |
| `wip` | Small local Rust extraction target |

Workspace dependencies also reveal future pressure: tree-sitter dependencies
are declared for Rust, C#, CSS, and Razor, but the implemented route described
below does not currently use them for extraction. See `Cargo.toml:33-37`.

## Graph Data Contract

The durable contract is relational:

- `workspaces` identifies a root URI and broad workspace kind.
- `extraction_runs` records provider, provider version, status, timestamps,
  optional git commit, and JSON properties.
- `files` stores URI, path, language, content hash, last-seen run, and JSON
  properties.
- `nodes` stores canonical node identity, language, kind, names, symbol key,
  source file, ranges, container, properties, first/last seen runs, and
  validity.
- `edges` stores canonical directed relation identity, source/target nodes,
  relation, context, confidence, score, weight, properties, first/last seen
  runs, and validity.
- `occurrences` stores source-level proof for nodes.
- `edge_evidence` stores source-level proof for edges.
- `node_search` is an FTS5 table, but current query code uses direct `LIKE`
  matching instead of this virtual table.

Evidence:

- Core schema is in
  `crates/semantic-graph-store/migrations/01_create_graph_store.sql:3-142`.
- The store exposes create/start/finish/upsert/insert/stats operations in
  `crates/semantic-graph-store/src/store/graph_store.rs:20-328` and
  `crates/semantic-graph-store/src/store/graph_store.rs:497`.
- FTS5 exists at
  `crates/semantic-graph-store/migrations/01_create_graph_store.sql:142`,
  while visualizer search uses `LIKE` predicates in
  `crates/semantic-graph-visualizer-server/src/query/graph_query_service.rs:528-571`.

Deep implication:

The store already has the right boundary between canonical graph state and
proof. The critical future design question is not whether evidence can be
stored; it is how new extraction routes will populate evidence without
collapsing distinct observations into the current canonical rows too early.

## Node Model

The implemented node model has two tiers:

- Durable schema fields: `kind`, `name`, `qualified_name`, `display_name`,
  `symbol_key`, `file_id`, full range, selection start, container node, and
  JSON properties.
- Extractor fields: provider, language, file URI, parent symbol key, detail,
  raw JSON, and LSP-derived range data.

See `crates/semantic-graph-store/migrations/01_create_graph_store.sql:35-57`
and `crates/semantic-graph-extract/src/model/extracted_symbol.rs:8-20`.

Current node kinds come from LSP `SymbolKind` normalization:

- `file`, `module`, `package`, `class`, `method`, `property`, `field`,
  `constructor`, `enum`, `interface`, `function`, `variable`, `constant`,
  `string`, `number`, `boolean`, `array`, `object`, `key`, `null`,
  `enum_member`, `struct`, `event`, `operator`, and `type_parameter`.

See `crates/semantic-graph-extract/src/document_symbols/mapper.rs:45-95`.

Rust-specific mapping currently converts file-structure node kinds into LSP
symbol kinds and then into the provider-neutral normalized kind. For example,
Rust traits become `interface`, impl blocks become `object`, variants become
`enum_member`, modules become `module`, fields become `field`, and functions
or macros become `function`. See
`crates/rust-analyzer-lib/src/semantic/document_symbols_for_file.rs:166-206`.

Deep implication:

The normalized kind set is intentionally provider-neutral, but it currently
inherits LSP naming tradeoffs. That works for a document-symbol containment
view. It will be too coarse for the durable semantic model once type-system
routes arrive. Examples:

- Rust traits should likely become `trait`, not only `interface`.
- Rust impl blocks need a first-class kind with target type/trait metadata.
- Rust modules and files are both structural containers but have different
  identity semantics.
- C# namespaces, classes, interfaces, records, structs, methods, properties,
  fields, events, and partial declarations will need language-specific
  normalization rules.

The durable schema can support this because `kind` is unconstrained text. The
constraint should be policy and tests, not a loose accidental vocabulary.

## ID Strategy

Durable node IDs and edge IDs are SHA256 hashes over typed identity parts:

- `node_id(workspace_id, language, symbol_key)`
- `edge_id(workspace_id, src_node_id, dst_node_id, relation, context)`

The hash helper includes the length of each part plus separators before the
part bytes, preventing ambiguous concatenation. See
`crates/semantic-graph-store/src/ids.rs:3-28`.

Provider symbol keys are human-readable strings before hashing. For document
symbols they include:

- file URI,
- normalized kind,
- selection range,
- symbol name,
- parent path.

See `crates/semantic-graph-extract/src/document_symbols/mapper.rs:11-34`.

File symbols use `file:{file_uri}`. File URIs require absolute paths and
percent-encode non-unreserved bytes. Source content hashes are SHA256 over file
contents. See
`crates/semantic-graph-extract/src/document_symbols/paths.rs:61-83` and
`crates/semantic-graph-extract/src/document_symbols/paths.rs:173-190`.

Deep implication:

The current identity is deterministic and good enough for hierarchical
document symbols. It is not yet a true semantic symbol identity. Including
selection range makes identity sensitive to edits that move a symbol. Including
parent path and name makes it vulnerable to rename/move churn. This is fine for
the present route because canonical rows are refreshed and evidence is
retained, but future definition/reference/call routes should prefer provider
symbol identity where available and treat source range as evidence, not
identity.

## Relation Vocabulary

The durable edge schema can store arbitrary relation strings plus optional
context. See
`crates/semantic-graph-store/migrations/01_create_graph_store.sql:59-75`.

The implemented extractor emits exactly one production relation:

- `contains`

Every document symbol receives a `contains` edge from its parent symbol when it
has one, or from the file node when it is top-level. Those edges are
`EXTRACTED`, score `1.0`, and include raw document-symbol JSON as evidence.
See
`crates/semantic-graph-extract/src/providers/rust_analyzer/rust_document_symbol_mapper.rs:81-131`
and
`crates/semantic-graph-extract/src/persist/extraction_persister.rs:261-284`.

Other relation names exist only in demo, tests, or UI styling:

- `calls` appears in demo seed and link styling.
- `references` appears in link styling.
- Test fixtures use `contains` with a `document-symbol` context.

See `crates/semantic-graph-store/src/store/graph_store.rs:369-496`,
`apps/SemanticGraph.Visualizer/src/SemanticGraph.Visualizer.Client/Services/GraphDiagramBuilder.cs:107-126`,
and
`crates/semantic-graph-visualizer-server/src/tests/projection.rs:24-117`.

Deep implication:

The current graph is a containment forest plus file roots. It is not yet a
behavioral graph. Algorithms over it will mostly find file/symbol nesting, not
execution flow, dependency flow, type usage, or architectural coupling.

Recommended next relation families:

- `defines`: file/package to symbol definition when containment needs to be
  distinguished from semantic ownership.
- `contains`: namespace/module/type/function lexical nesting.
- `references`: textual or semantic symbol reference.
- `calls`: executable call edge.
- `imports`: module/import dependency.
- `implements`: type or impl block implements trait/interface.
- `inherits`: type inheritance/base-type relation.
- `has_type`: variable/field/parameter return type edge.
- `overrides`: method/property override.
- `uses_attribute`: attribute/annotation usage.

The schema already supports this. The main missing piece is route-specific
extraction and evidence policy.

## Confidence Model

The durable edge schema constrains confidence to:

- `EXTRACTED`
- `INFERRED`
- `AMBIGUOUS`

See `crates/semantic-graph-store/migrations/01_create_graph_store.sql:65-68`.

Current production document-symbol edges are always:

- confidence: `EXTRACTED`
- confidence score: `1.0`
- weight: `1.0`

See
`crates/semantic-graph-extract/src/providers/rust_analyzer/rust_document_symbol_mapper.rs:107-109`
and
`crates/semantic-graph-extract/src/persist/extraction_persister.rs:261-276`.

Deep implication:

The confidence model exists but has not been exercised. That is appropriate
for the current route because document-symbol containment is directly returned
by the provider. Future routes should not blindly stamp everything
`EXTRACTED`. A useful policy would be:

- `EXTRACTED`: directly returned by a semantic provider route with source
  range proof.
- `INFERRED`: derived by local joining, normalization, package metadata, graph
  analysis, or route composition.
- `AMBIGUOUS`: provider result is partial, name-only, unresolved, overloaded,
  generated, or conflicts with other evidence.

Confidence should remain an edge property, while evidence rows should explain
why the edge earned that label.

## Context Model

The schema includes an optional `context` column on edges and makes it part of
the edge uniqueness contract. See
`crates/semantic-graph-store/migrations/01_create_graph_store.sql:59-75` and
`crates/semantic-graph-store/src/ids.rs:7-20`.

The current production persister always writes `context: None` for
document-symbol relations. See
`crates/semantic-graph-extract/src/persist/extraction_persister.rs:261-276`.
The visualizer fixture uses `context = 'document-symbol'`, and edge details
return context to the client. See
`crates/semantic-graph-visualizer-server/src/tests/projection.rs:24-117` and
`crates/semantic-graph-visualizer-server/src/dto/graph_edge_details_dto.rs:8-15`.

Deep implication:

Context is currently underused. It should become the route/subrelation
dimension that prevents different facts from collapsing into one edge. For
example:

- `contains` with `lexical`
- `contains` with `module_tree`
- `references` with `read`
- `references` with `write`
- `calls` with `static_dispatch`
- `calls` with `dynamic_dispatch`
- `has_type` with `parameter`
- `has_type` with `return`

Using context this way preserves a compact relation vocabulary without losing
semantic distinction.

## Extraction Semantics

The extraction CLI exposes three implemented commands:

- `rust-document-symbols`
- `rust-crate-document-symbols`
- `rust-workspace-document-symbols`

See `crates/semantic-graph-extract/src/main.rs:26-46`.

The route flow is:

1. Validate and canonicalize workspace, package, and source paths.
2. Discover Rust source files for package or workspace routes.
3. Ask the in-process Rust facade for document symbols.
4. Map hierarchical LSP `DocumentSymbol[]` into provider-neutral symbols and
   relations.
5. Persist a run, files, file nodes, symbol nodes, definition occurrences,
   containment edges, and edge evidence.

Evidence:

- Path validation canonicalizes paths, verifies package/source boundaries,
  sorts and deduplicates batch file paths, and normalizes relative paths with
  forward slashes. See
  `crates/semantic-graph-extract/src/document_symbols/paths.rs:9-58` and
  `crates/semantic-graph-extract/src/document_symbols/paths.rs:104-170`.
- The Rust provider discovers package and workspace files, runs batch
  extraction, attaches provider version, and records raw metadata. See
  `crates/semantic-graph-extract/src/providers/rust_analyzer/rust_analyzer_provider.rs:30-141`.
- The mapper rejects flat `SymbolInformation[]` responses and requires
  hierarchical `DocumentSymbol[]`. See
  `crates/semantic-graph-extract/src/providers/rust_analyzer/rust_document_symbol_mapper.rs:23-40`
  and
  `crates/semantic-graph-extract/src/tests/rust_document_symbol_mapper.rs:1-33`.
- Persistence starts one run per single-file or batch extraction and marks it
  `complete` or `failed`. See
  `crates/semantic-graph-extract/src/persist/extraction_persister.rs:18-63`
  and
  `crates/semantic-graph-extract/src/persist/extraction_persister.rs:100-106`.

Deep implication:

The extraction architecture is already route-shaped, but the route model is
implicit in code rather than durable schema. There is an `extraction_runs`
table, but there is no child table for per-route status, per-file route
freshness, request parameters, route-specific errors, or route capabilities.
That will matter as soon as document symbols are joined with references,
definitions, implementations, call hierarchy, type hierarchy, package
metadata, or C# routes.

## Rust Facade Semantics

The Rust facade loads a Cargo workspace through pinned libraries, discovers
workspace member packages, and collects module source files by walking module
declarations from target roots. See
`crates/rust-analyzer-lib/src/project/load_workspace.rs:15-53`,
`crates/rust-analyzer-lib/src/project/load_workspace.rs:57-129`, and
`crates/rust-analyzer-lib/src/project/load_workspace.rs:160`.

Document symbols are built from file structure:

- It loads an analysis database for the workspace.
- It resolves a file ID from the VFS.
- It calls `file_structure` with `exclude_locals: true`.
- It converts ranges to UTF-16 LSP positions.
- It rebuilds parent/child hierarchy from structure node parent indices.

See `crates/rust-analyzer-lib/src/semantic/document_symbols_for_file.rs:26-109`
and
`crates/rust-analyzer-lib/src/semantic/document_symbols_for_file.rs:129-160`.

Deep implication:

The facade uses semantic project loading, but the extracted fact is still file
structure. That makes the route more authoritative than raw parsing for
workspace membership and file inclusion, but it does not yet resolve references
or calls. The phrase "library-backed extraction" should not be overread as
"full semantic extraction." Today it is structurally semantic: the project
model is semantic, the symbol hierarchy is provider-backed, and the relations
are containment-only.

## Persistence Lifecycle

The storage lifecycle is split cleanly:

- `workspaces` and `files` are upserted.
- `nodes` and `edges` are canonical rows and are upserted by deterministic ID.
- `occurrences` and `edge_evidence` are appended every run.
- `first_seen_run_id`, `last_seen_run_id`, and `valid_to_run_id` exist on
  canonical rows.
- Upserts reset `valid_to_run_id` to `NULL`.
- No implemented route currently marks stale rows by setting `valid_to_run_id`.

Evidence:

- File upsert uses `(workspace_id, uri)` conflict resolution. See
  `crates/semantic-graph-store/src/store/graph_store.rs:116-145`.
- Node upsert updates canonical fields and resets validity. See
  `crates/semantic-graph-store/src/store/graph_store.rs:160-230`.
- Edge upsert updates confidence/weight/properties and resets validity. See
  `crates/semantic-graph-store/src/store/graph_store.rs:235-289`.
- Occurrences and evidence are inserted, not upserted. See
  `crates/semantic-graph-store/src/store/graph_store.rs:291-366`.

Deep implication:

The system already distinguishes current best graph state from historical
proof. That is the right durability stance. The missing lifecycle piece is
negative observation: a later run that fails to see a previous node or edge
should be able to close it with `valid_to_run_id` without deleting its
evidence. The schema is ready; the extractor does not yet implement stale-row
handling.

## Tracked Data Classes

The repository currently tracks these data classes:

| Data class | Current implementation | Lifecycle |
| --- | --- | --- |
| Workspace | `workspaces.root_uri`, `kind` | Upserted by extraction |
| Extraction run | `extraction_runs` | One per single-file or batch persistence call |
| Source file | `files.uri`, `path`, `language`, `content_hash` | Upserted per extracted file |
| Canonical node | `nodes` | Upserted by hash ID |
| Canonical edge | `edges` | Upserted by hash ID |
| Node proof | `occurrences` | Appended per run |
| Edge proof | `edge_evidence` | Appended per run |
| Provider payload | `raw_json`, `properties_json` | Stored as JSON text |
| Search index | `node_search` FTS5 | Present but not populated by current code |
| Visualizer projection | JSON-RPC DTOs | Derived on demand |
| Diagram state | Blazor node/link models | Client-local, regenerated on load |
| Smoke output | Temp DBs and console counts | Validation only |

Deep implication:

The architecture has the right storage boundary, but route freshness is too
coarse. `files.last_seen_run_id` tells when a file participated in a run. It
does not say whether references, calls, definitions, implementations, semantic
tokens, or type hierarchy were fresh for that file. Future semantic modelling
should add route-level freshness before adding many new edge types.

## Query And RPC Model

The visualizer backend exposes four JSON-RPC 2.0 methods:

- `graph.projection`
- `graph.node_details`
- `graph.edge_details`
- `graph.search_nodes`

See `crates/semantic-graph-visualizer-server/src/rpc/rpc_handler.rs:45-48`.

Projection behavior:

- Opens SQLite read-only.
- Selects a limited number of non-file, non-stale symbols.
- Adds the file nodes for selected symbols.
- Returns only edges whose endpoints are both included.
- Defaults to 150 symbols and caps projection limit at 1000.

See
`crates/semantic-graph-visualizer-server/src/query/sqlite_read_pool.rs:9-16`,
`crates/semantic-graph-visualizer-server/src/query/graph_query_service.rs:34-48`,
`crates/semantic-graph-visualizer-server/src/query/graph_query_service.rs:126-243`,
and
`crates/semantic-graph-visualizer-server/src/dto/graph_projection_params_dto.rs:5-20`.

Details behavior:

- Node details include ranges, container, first/last run IDs, properties,
  incoming/outgoing counts, relation summaries, and occurrences.
- Edge details include context, confidence, weight, first/last run IDs,
  properties, source endpoint, target endpoint, and evidence.

See
`crates/semantic-graph-visualizer-server/src/query/graph_query_service.rs:53-111`,
`crates/semantic-graph-visualizer-server/src/query/graph_query_service.rs:265-411`,
and
`crates/semantic-graph-visualizer-server/src/query/graph_query_service.rs:477-526`.

Search behavior:

- Searches node name, display name, qualified name, and file path.
- Uses escaped `LIKE` patterns rather than the FTS table.
- Defaults to 25 results and caps search at 50.

See
`crates/semantic-graph-visualizer-server/src/query/graph_query_service.rs:528-611`
and
`crates/semantic-graph-visualizer-server/src/dto/graph_search_nodes_params_dto.rs:5-34`.

Deep implication:

The backend is intentionally a projection service, not a general graph query
engine. That is good for the current UI, but future graph analysis will need a
separate query/projection layer that can express relation families, traversal
depth, file/package filters, confidence filters, stale-row handling, and
algorithm-specific graph views.

## UI Projection Model

The Blazor client keeps three models separate:

- JSON-RPC DTO records from the backend.
- Blazor.Diagrams node/link models used for the viewport.
- Selection/detail state used by the inspector.

Evidence:

- `GraphClient` sends typed JSON-RPC requests for projection, node details,
  edge details, and search. See
  `apps/SemanticGraph.Visualizer/src/SemanticGraph.Visualizer.Client/Services/GraphClient.cs:19-99`.
- `GraphDiagramBuilder` clears and rebuilds diagram nodes and links from a
  projection DTO. See
  `apps/SemanticGraph.Visualizer/src/SemanticGraph.Visualizer.Client/Services/GraphDiagramBuilder.cs:13-78`.
- `SemanticGraphNodeModel` locks diagram nodes, controls size, and adds ports.
  See
  `apps/SemanticGraph.Visualizer/src/SemanticGraph.Visualizer.Client/Models/SemanticGraphNodeModel.cs:9-25`.
- `Home` enables Blazor.Diagrams virtualization, disables multi-selection,
  handles selection, fetches details on demand, and maintains separate search
  state. See
  `apps/SemanticGraph.Visualizer/src/SemanticGraph.Visualizer.Client/Pages/Home.razor.cs:15-16`,
  `apps/SemanticGraph.Visualizer/src/SemanticGraph.Visualizer.Client/Pages/Home.razor.cs:87-154`,
  `apps/SemanticGraph.Visualizer/src/SemanticGraph.Visualizer.Client/Pages/Home.razor.cs:185-249`,
  and
  `apps/SemanticGraph.Visualizer/src/SemanticGraph.Visualizer.Client/Pages/Home.razor.cs:251-334`.
- The Razor view has toolbar search, stats, refresh, diagram canvas, node
  detail tabs, occurrence tabs, edge detail tabs, evidence tabs, and raw JSON
  tabs. See
  `apps/SemanticGraph.Visualizer/src/SemanticGraph.Visualizer.Client/Pages/Home.razor:15-86`
  and
  `apps/SemanticGraph.Visualizer/src/SemanticGraph.Visualizer.Client/Pages/Home.razor:136-303`.

Deep implication:

The UI already respects the core architecture: durable graph records stay in
SQLite, backend DTOs are read models, and diagram models are presentation
objects. The current layout is deterministic and simple, organized by file row
and symbol slot. That is acceptable for a containment graph, but richer
relations will quickly need relation-aware layout and filtering.

## Graph Theory Model

The implemented graph is a directed property graph with evidence side tables.
It is not yet a multigraph in SQL uniqueness terms, but it can represent
parallel semantic distinctions when `relation` or `context` differs.

Current graph-theory shape:

- File nodes are roots.
- Symbol nodes form hierarchical trees under file nodes.
- Each symbol has one incoming `contains` edge from its immediate container.
- Edges are directed from container to contained symbol.
- There are no implemented cross-file semantic edges in the production route.
- There are no implemented hyperedges.
- There are no persisted algorithmic projections such as communities,
  centrality, layout, cycles, or component summaries.

### Containment Forest

The current production graph is a forest of containment trees, one tree per
source file, with possible nested symbol nodes. This is confirmed by the
document-symbol mapper and smoke counts: every symbol becomes exactly one
`contains` relation, and each file also becomes a file node.

Graph-theory consequence:

- Degree mostly measures nesting breadth, not architectural influence.
- Betweenness mostly finds lexical containers, not dependency bridges.
- Connected components mostly correspond to files unless future cross-file
  edges are added.
- Shortest paths are useful for containment ancestry but not for behavior.
- Cycle detection is not meaningful on the current production route because
  containment should be acyclic.

### Directed Edges

The durable edge table has explicit `src_node_id` and `dst_node_id`, so
direction is first-class. See
`crates/semantic-graph-store/migrations/01_create_graph_store.sql:59-75`.

Current direction convention:

- source = container,
- target = contained symbol.

Deep implication:

Direction should be preserved per relation family. A future `calls` edge and a
future `contains` edge both use source/target, but the meaning of source and
target is relation-specific. Query and UI labels should never assume every
edge means parent-to-child.

### Multigraph Pressure

The edge uniqueness rule is:

```text
workspace_id, src_node_id, dst_node_id, relation, context
```

See `crates/semantic-graph-store/migrations/01_create_graph_store.sql:74`.

That means the store can represent multiple edge types between the same pair
if relation or context differs. It cannot represent two independent pieces of
canonical evidence as separate canonical edges with the same relation/context;
those are intentionally accumulated in `edge_evidence`.

Deep implication:

This is a good canonical multigraph compromise. The canonical edge is the
claim; `edge_evidence` is the bag of observations supporting the claim. If a
future route needs two semantically distinct edges with the same endpoints and
relation, context must carry the distinction.

### Projection Discipline

The visualizer does not render the full database. It applies a stable bounded
projection: selected symbols, their files, and fully internal edges. See
`crates/semantic-graph-visualizer-server/src/query/graph_query_service.rs:126-243`.

Graph-theory consequence:

Algorithms or visual impressions from the current UI are about the projection,
not the whole graph. Missing nodes may remove edges and disconnect regions.
That is correct for UI responsiveness, but any future analytical metric must
record whether it ran on the full graph or a projection.

## Analysis Readiness

The schema has placeholders for analysis-ready graph data:

- edge confidence and score,
- edge weight,
- current/stale validity,
- provider raw JSON,
- source ranges,
- node search table,
- file content hashes,
- run provenance.

The code does not yet implement:

- centrality computation,
- community detection,
- cycle detection,
- dependency projections,
- call graph projections,
- package/module summaries,
- layout persistence,
- stale-row closing,
- CSV/Data Package snapshots,
- hyperedges.

See `README.md:346-354` for the current missing features list.

Deep implication:

The repository is at the correct stage for adding semantic routes before
adding global graph algorithms. Running centrality or community detection on a
pure containment graph would produce mechanically valid but low-value results.
The next analytical milestone should be a relation-rich graph, not an
algorithm-heavy graph.

## Quality And Validation Model

The workspace enforces strict lint expectations:

- `unwrap_used = deny`
- `expect_used = deny`
- `panic = deny`
- `unused_must_use = deny`

See `Cargo.toml:51-57`.

Validation surfaces:

- Store tests cover schema creation, deterministic IDs, upsert behavior, and
  foreign-key enforcement.
- Extract tests cover path validation, symbol mapping, fixture persistence,
  batch persistence, and provider error formatting.
- Smoke tests cover the Rust facade route, crate extraction route, workspace
  extraction route, persistence, and submodule exclusion.
- Visualizer backend tests cover projection, node details, edge details,
  search, and JSON-RPC error handling.
- The Blazor client builds through the solution.

Evidence:

- Store tests: `crates/semantic-graph-store/src/tests/store.rs:1-140`.
- Extract tests:
  `crates/semantic-graph-extract/src/tests/document_symbol_pipeline.rs:1-280`,
  `crates/semantic-graph-extract/src/tests/document_symbols_paths.rs:1-36`,
  and
  `crates/semantic-graph-extract/src/tests/rust_document_symbol_mapper.rs:1-33`.
- Visualizer backend tests:
  `crates/semantic-graph-visualizer-server/src/tests/projection.rs:24-207`.
- Useful validation commands are listed in `README.md:313-320`.

Deep implication:

The repository has unusually good route-level smoke coverage for its size.
The missing coverage will emerge with new relation families: every new route
should have fixture mapping tests, persistence tests, smoke counts, and
visualizer detail tests before it is treated as part of the durable semantic
model.

## Design Strengths

- Durable storage is already normalized around graph state and proof state.
- Deterministic IDs make upserts stable across repeated runs.
- Raw provider payloads are retained in JSON properties/evidence.
- Extraction run status is explicit.
- Provider-neutral extraction structs keep language-specific mapping out of
  the store.
- Rust project loading is in-process and tested through workspace smoke routes.
- The visualizer backend is read-only and bounded.
- The UI separates DTOs, diagram models, and inspector state.
- The codebase uses typed error enums with location capture.
- Validation surfaces exercise the end-to-end Rust route.

## Design Risks

- Document-symbol identity is range-sensitive and will churn under edits.
- Route freshness is file-level/run-level, not per semantic capability.
- `valid_to_run_id` exists but stale-row closing is not implemented.
- `node_search` exists but is not populated or queried by current search.
- `context` exists but production extraction writes `None`.
- C# language modelling exists only as enum/provider names and schema values,
  not as an implemented route.
- Tree-sitter dependencies are present but not part of current extraction.
- File/package/crate concepts are under-modelled: files are nodes, but crates,
  packages, targets, modules, and workspaces are not yet durable graph nodes.
- The visualizer projection is deterministic but not graph-layout aware.
- Relation styling anticipates `calls` and `references`, but production data
  currently contains only containment relations.

## Recommended Next Semantic Slice

The next modelling slice should add one semantic relation family end to end,
not several partial families. The best candidate is Rust references or calls,
because the store and UI already support edge details, evidence, confidence,
and directed projection.

A durable slice should include:

- route model: explicit route name and provider method;
- provider mapping: raw provider payload to provider-neutral facts;
- identity policy: stable symbol keys that do not depend only on source range;
- persistence: canonical edges plus source evidence;
- stale handling: close rows absent from a later route run;
- tests: mapper fixture, persistence fixture, smoke count, query/detail test;
- UI: relation filter or visual distinction only after the data exists.

For route freshness, add a table conceptually shaped like:

```text
source_file_route_status(
  file_id,
  route,
  provider,
  provider_version,
  content_hash,
  last_started_run_id,
  last_complete_run_id,
  last_status,
  diagnostics_json
)
```

This would prevent document-symbol freshness from being confused with
reference, call, implementation, type, or semantic-token freshness.

## Open Questions

- Should canonical Rust symbol identity move from document-symbol range keys to
  provider-resolved symbol IDs once richer routes are added?
- Should `contains` be split into lexical containment and file definition
  relations?
- Should C# partial declarations become one canonical node with multiple
  definition occurrences?
- Should generated files and macro-expanded Rust facts be marked by route,
  confidence, context, or source-file metadata?
- Should FTS5 become the primary search path, and if so, how should it be kept
  in sync with node upserts?
- Should graph layouts be persisted in SQLite or treated as client-local
  presentation state?
- How should route-specific stale-row closing interact with failed runs?
- Which relation families should be available before centrality/community
  analysis is considered meaningful?
