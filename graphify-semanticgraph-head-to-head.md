# Graphify And SemanticGraph Head-To-Head Analysis

Status: Research note

Date: 2026-06-14

Compared documents:

- `graphify-semantic-model-analysis.md`
- `semanticgraph-codebase-analysis.md`

## Purpose

This note compares the two prior analysis documents head to head. It is not an
ADR. It does not replace either source analysis. Its job is to make the design
tradeoffs explicit: where Graphify is stronger, where SemanticGraph is already
stronger, where they solve different problems, and what this means for the
next semantic modelling work.

The short version:

- Graphify is much broader in semantic vocabulary, product behavior, graph
  analysis, artifact lifecycle, and export/query ergonomics.
- SemanticGraph is much cleaner as a durable source-of-truth design: SQLite
  canonical rows, evidence tables, extraction runs, directed edges, and a
  read-only projection API. It now also has route status, route observations,
  stale closing, and provider-backed Rust references.
- Graphify is a mature generated graph artifact system; SemanticGraph is a
  younger durable semantic graph store and extraction pipeline.
- Graphify has richer graph theory in practice; SemanticGraph has a better
  foundation for making graph theory durable and auditable.
- The right synthesis is not to port Graphify wholesale. It is to bring over
  Graphify's proven categories, lifecycle pressure, and projection discipline
  while preserving SemanticGraph's database/evidence boundary.

## Baseline

The two systems are not equivalent products.

Graphify, as analyzed, has two semantic layers:

- deterministic code-structure extraction;
- LLM-driven corpus semantic extraction for docs, papers, images, and
  transcripts.

See `graphify-semantic-model-analysis.md:57-79`.

SemanticGraph, as analyzed, currently implements two Rust production extraction
families:

- Rust document-symbol extraction into a durable SQLite containment graph.
- Rust workspace references into directed `references` edges, reference
  occurrences, edge evidence, route observations, and stale closing.

Its measured workspace document-symbol route is 147 files, 1190 symbols, 1337
persisted nodes, 1190 `contains` edges, 1190 occurrences, and 1190
edge-evidence rows. Its measured workspace references route adds 2484
`references` edges and 2968 reference occurrences, for 4158 total occurrence
and evidence rows. See `semanticgraph-codebase-analysis.md`.

That means the fair comparison is:

- Graphify: broad, artifact-oriented, analysis-rich, but looser as durable
  evidence.
- SemanticGraph: narrower than Graphify, but now relation-rich for Rust
  containment plus references and stronger as durable evidence storage.

## Scorecard

| Dimension | Graphify | SemanticGraph | Head-to-head result |
| --- | --- | --- | --- |
| Semantic breadth | Code structure, docs, papers, images, transcripts, rationale, concepts, similarity, hyperedges | Rust document-symbol containment and provider-backed Rust references | Graphify is still broader; SemanticGraph is now deeper for one code relation |
| Semantic authority for Rust/C# code | Syntax-first code extraction with heuristic/name-based resolution | Provider-backed Rust project/file-structure and references through pinned libraries | SemanticGraph has the better authority model for implemented Rust facts |
| Durable source of truth | NetworkX node-link JSON plus sidecars and caches | SQLite normalized schema with runs, files, nodes, edges, occurrences, evidence | SemanticGraph is stronger |
| Evidence retention | Source fields and raw-ish artifacts exist, but merge/dedup can overwrite/collapse | Canonical graph rows separated from append-only occurrence/evidence rows | SemanticGraph is stronger |
| Relation richness | Broad relation vocabulary, contexts, semantic/document edges, domain labels | Production relations are `contains` and `references` | Graphify is richer overall; SemanticGraph now has a real semantic edge family |
| Confidence model | Labels plus scores used in extraction, export, tests, and analysis | Labels and scores in schema; `contains` is `EXTRACTED`, references can be `EXTRACTED` or `AMBIGUOUS` | Graphify exercises it more broadly; SemanticGraph now exercises it durably |
| Direction | Directional semantics often recovered from `_src`/`_tgt` because default graph is undirected | Directed edges are first-class SQL columns | SemanticGraph is stronger |
| Multigraph pressure | Simple graph pressure; dedicated MultiDiGraph compatibility probing | Unique edge claim by source, target, relation, context; evidence accumulates separately | SemanticGraph has the cleaner canonical model |
| Tracked data lifecycle | Rich lifecycle under `graphify-out`: manifest, caches, conversions, fragments, graph JSON, telemetry | Explicit route status, route observations, runs, stale closing, canonical rows, and proof rows | SemanticGraph has the cleaner durable lifecycle; Graphify exposes more product lifecycle pressure |
| Graph algorithms | Communities, cohesion, god nodes, betweenness, surprise ranking, cycles, hyperedge projection pressure | No centrality/community/cycle algorithms yet; graph theory section is readiness analysis | Graphify is stronger today |
| Projection discipline | Many product projections, but canonical artifact also mixes derived fields | Read-only bounded projection API; no analytical projections yet | SemanticGraph is cleaner; Graphify is more mature |
| Query/search | Graph-file query server with IDF scoring, BFS/DFS behavior, telemetry | JSON-RPC projection/details/search over read-only SQLite; search is simple `LIKE` | Graphify is richer; SemanticGraph is better integrated with evidence |
| UI/export | HTML, report, Obsidian, canvas, graph formats, assistant hooks | Blazor.Diagrams/Radzen read-only viewport and inspector | Graphify is broader; SemanticGraph is more aligned to durable inspection |
| Validation | Broad tests around extraction behavior, confidence, hyperedges, analysis | Strong route smoke tests and storage/backend tests for current slice | Tie by maturity-adjusted scope |
| Biggest risk | Artifact merges/projections can be mistaken for durable truth | Rust references can be overread as full semantic modelling without calls/types/imports | Different risks |

## Core Difference

Graphify optimizes for a useful generated graph artifact. SemanticGraph
optimizes for a durable, auditable graph store.

Graphify's public contract is intentionally simple: nodes have IDs, labels,
file types, source files, and optional source locations; edges have source,
target, relation, confidence, source file, and optional scoring/context fields;
hyperedges live at top level or graph metadata. See
`graphify-semantic-model-analysis.md:81-106`.

SemanticGraph's contract is normalized: workspaces, extraction runs, files,
nodes, edges, occurrences, edge evidence, and search scaffolding are separate.
See `semanticgraph-codebase-analysis.md`.

That is the architectural split:

- Graphify collapses many useful concepts into a portable graph artifact.
- SemanticGraph separates durable graph claims from proof and projection.

Neither shape is universally better. For a tool that must produce a map for
humans and agents quickly, Graphify's artifact model is pragmatic. For a
long-lived semantic database that must survive incremental refresh, provider
changes, and conflicting evidence, SemanticGraph's model is the stronger base.

## Extraction Authority

Graphify extracts code structure locally and deterministically, but the
analysis says its code graph is mostly syntax-first. It also has a separate
LLM semantic layer mainly for non-code corpus material. See
`graphify-semantic-model-analysis.md:57-79`.

SemanticGraph uses narrower Rust routes, but they are backed by pinned Rust
project loading, file structure, and `analysis.find_all_refs` through
`rust-analyzer-lib`. Document symbols are structurally semantic; references are
resolved semantic facts. Calls, types, imports, implementations, and C# facts
are not yet implemented. See `semanticgraph-codebase-analysis.md`.

Head-to-head:

- Graphify has more extraction categories today.
- SemanticGraph has the better authority model for the Rust facts it does
  implement.
- SemanticGraph should not chase broad syntax-first extraction just to match
  Graphify's vocabulary count.
- Graphify's extraction breadth should be treated as a requirements catalog,
  not as an implementation authority.

## Graph Contract

Graphify's graph contract is intentionally lightweight and export-friendly.
The downside is that node kind, relation semantics, source proof, analysis
state, and product fields can live in the same artifact namespace. See
`graphify-semantic-model-analysis.md:81-106` and
`graphify-semantic-model-analysis.md:768-798`.

SemanticGraph's graph contract is database-first. Canonical nodes and edges
are separate from occurrence and edge evidence. The source analysis calls this
the right boundary, and VS-10 added route status and route observations for
freshness and stale closing. See `semanticgraph-codebase-analysis.md`.

Head-to-head:

| Concern | Graphify | SemanticGraph | Better base |
| --- | --- | --- | --- |
| Portable generated output | Strong | Not yet implemented | Graphify |
| Durable source of truth | Weak-to-medium | Strong | SemanticGraph |
| Source proof separation | Partial | Strong | SemanticGraph |
| Derived projection separation | Mixed | Cleaner, but less mature | SemanticGraph |
| Human-readable artifact | Strong | Weak today | Graphify |

Implementation implication:

SemanticGraph should keep SQLite as the truth and add exports as rebuildable
products. It should not make a generated JSON or UI projection the canonical
state.

## Node Model

Graphify uses broad corpus categories and naming conventions rather than a
strong first-class node-kind model. Its node model relies on `file_type`,
labels, source fields, metadata, and ID conventions. See
`graphify-semantic-model-analysis.md:108-140`.

SemanticGraph has explicit durable node fields and provider-neutral extractor
fields. It normalizes LSP symbol kinds into strings such as `file`, `module`,
`function`, `method`, `field`, `struct`, `enum`, `interface`, and
`type_parameter`. Its own analysis calls out that this LSP-derived vocabulary
will be too coarse for future type-system routes. See
`semanticgraph-codebase-analysis.md`.

Head-to-head:

- Graphify is flexible but under-typed.
- SemanticGraph is more structured but still too generic for serious Rust/C#
  type modelling.
- Graphify's concept/document/media node pressure is useful for future
  non-code evidence.
- SemanticGraph should split source category, language, node kind, and
  provider metadata instead of using one field for all of them.

SemanticGraph should move toward an explicit semantic-kind vocabulary:

- workspace/package/crate/target/file/module/namespace;
- type/trait/interface/impl block;
- function/method/constructor/property/field/event;
- enum variant/parameter/local/type parameter;
- document/concept/rationale only if non-code corpus extraction is added.

## Identity Model

Graphify's IDs are portable strings derived from file stems and entity labels.
The source analysis calls that adequate for a generated JSON artifact but not
robust enough for durable Rust/C# symbol identity. See
`graphify-semantic-model-analysis.md:143-168`.

SemanticGraph hashes typed identity parts for durable node and edge IDs. It
still uses document-symbol keys that include file URI, kind, selection range,
name, and parent path. The analysis flags this as deterministic but
range-sensitive and rename/move-sensitive. See
`semanticgraph-codebase-analysis.md`.

Head-to-head:

- Graphify identity is more readable and portable.
- SemanticGraph identity is safer for canonical storage because IDs are typed,
  hashed, and scoped by workspace.
- Both identity schemes have churn risk.
- SemanticGraph's next step should be provider-resolved symbol identity where
  available, with source ranges stored as evidence rather than identity.

## Relation Vocabulary

Graphify is far richer today. Its nominal relation set includes `inherits`,
`implements`, `references`, `calls`, `imports`, `imports_from`,
`re_exports`, `contains`, and `method`, while observed emitted relations are
broader and include structure, type, dependency, call, construction, domain,
LLM/concept, and hyperedge relation families. See
`graphify-semantic-model-analysis.md:170-237`.

SemanticGraph production extraction emits two relations:

- `contains`
- `references`

The schema and UI still anticipate more relation families, but production data
is no longer only containment. See `semanticgraph-codebase-analysis.md`.

Head-to-head:

- Graphify still wins on relation breadth.
- SemanticGraph wins on evidence-preserving storage for relation claims.
- Graphify's vocabulary is a good discovery catalog, but too open-ended to
  adopt directly as canonical storage.
- SemanticGraph should create a controlled relation table or enum, with
  context for subtyping and extension metadata for framework/domain specifics.

Useful synthesis:

| Relation family | Graphify status | SemanticGraph status | Recommended treatment |
| --- | --- | --- | --- |
| Containment | Implemented broadly | Implemented for document symbols | Keep, but define lexical/file/module meanings |
| Imports/dependencies | Implemented in places | Not implemented | Add as separate projection-friendly family |
| Calls | Implemented syntax/heuristic side | Not implemented | Add via provider-backed route, not name matching |
| References/types | Broad contexts | Rust references implemented; type edges not implemented | Extend context/type policy after reference identity stabilizes |
| Inheritance/implementation | Present | Not implemented | Add once type identity is stronger |
| Conceptual/similarity | Present | Not implemented | Defer until document/concept corpus exists |
| Hyperedges | Present outside core edge topology | Not implemented | Defer or model with first-class tables |

## Context And Subtyping

Graphify uses relation plus context to carry useful details such as field,
parameter type, return type, generic argument, attribute, value, and type. See
`graphify-semantic-model-analysis.md:238-265`.

SemanticGraph already has an edge `context` column and makes context part of
edge uniqueness. Document-symbol `contains` still writes `None`, while
`references` writes `context = 'symbol'`. See
`semanticgraph-codebase-analysis.md`.

Head-to-head:

- Graphify demonstrates why context matters.
- SemanticGraph has the better storage slot for context and now uses it for
  references.
- SemanticGraph still needs broader route policy before adding many relation
  types.

Implementation rule:

Use relation for the broad edge family and context for query-relevant
subtypes. Do not bury subtype semantics only in JSON.

## Confidence

Both systems use the same confidence labels:

- `EXTRACTED`
- `INFERRED`
- `AMBIGUOUS`

Graphify exercises the model more deeply: it has labels, confidence scores,
tests, export defaults, and analysis behavior that treats uncertain edges as
noteworthy. See `graphify-semantic-model-analysis.md:267-306` and
`graphify-semantic-model-analysis.md:958-993`.

SemanticGraph has the labels and scores in schema. Document-symbol containment
is `EXTRACTED` with score `1.0`; references are `EXTRACTED` with score `1.0`
when source and target symbols resolve, or `AMBIGUOUS` with score `0.6` when
the source falls back to a file node. See `semanticgraph-codebase-analysis.md`.

Head-to-head:

- Graphify has broader confidence behavior.
- SemanticGraph has a cleaner place to store confidence as durable edge
  metadata and now exercises ambiguity for file-fallback references.
- SemanticGraph should preserve the distinction between epistemic confidence
  and graph-analysis weight. Graphify's graph-theory analysis explicitly shows
  why they should not be collapsed.

## Lifecycle And Tracked Data

Graphify exposes a complete lifecycle pressure map:

- output artifact,
- report,
- manifest,
- converted sidecars,
- extraction fragments,
- AST cache,
- semantic cache,
- analysis sidecars,
- watch/hook state,
- local query telemetry.

See `graphify-semantic-model-analysis.md:555-591`.

Its manifest has separate AST and semantic freshness concepts, and missing
semantic hashes requeue semantic extraction. See
`graphify-semantic-model-analysis.md:593-623`.

SemanticGraph tracks fewer product lifecycle classes, but does so with clearer
boundaries: workspace, extraction run, route status, route observation, source
file, canonical node, canonical edge, node proof, edge proof, provider
payload, search index, projection DTOs, diagram state, and smoke output. See
`semanticgraph-codebase-analysis.md`.

Head-to-head:

| Lifecycle concern | Graphify | SemanticGraph | Practical conclusion |
| --- | --- | --- | --- |
| Route freshness | AST/semantic distinction exists in manifest | DB route status and observations implemented for Rust document symbols/references | Extend the pattern per new route |
| Raw evidence | Present but mixed with artifact/cache behavior | Separate occurrence/evidence tables | Preserve SemanticGraph boundary |
| Cache | Mature and cost-aware | Minimal/not central | Add caches only as disposable acceleration |
| Generated outputs | Mature | Minimal | Add exports after DB truth is stable |
| Telemetry | Query logging exists | No comparable telemetry | Keep telemetry local/opt-in if added |
| Stale handling | Merge/prune behavior exists | DB stale closing implemented for Rust document symbols/references | Define ownership carefully for future routes |

Deep conclusion:

Graphify shows the operational states a real graph tool accumulates.
SemanticGraph shows where those states should live if durability matters.
The synthesis is route-level database state plus optional rebuildable
artifacts, not a large `graph-out` directory as truth.

## Merge, Deduplication, And Evidence

Graphify has pragmatic merge/dedup behavior for generated artifacts. It can
merge new chunks with an existing graph, prune sources, avoid accidental
shrinkage, restore direction markers, and preserve hyperedges during export.
The analysis still warns that the artifact is not event-sourced and can
overwrite, repair, dedup, or prune. See
`graphify-semantic-model-analysis.md:734-766`.

SemanticGraph's canonical rows are upserted, while occurrence and edge
evidence rows are appended every run. It has `first_seen_run_id`,
`last_seen_run_id`, `valid_to_run_id`, route observations, and implemented
stale-row closing for document-symbol and reference routes. See
`semanticgraph-codebase-analysis.md`.

Head-to-head:

- Graphify has a more complete artifact update story.
- SemanticGraph has a more correct durability story.
- SemanticGraph has implemented the basic negative-observation path; future
  routes need route-specific ownership rules.
- Graphify's shrinkage/prune protections are a useful product idea, but in
  SemanticGraph they should become run/state transitions, not artifact guards.

## Graph Theory

Graphify's graph-theory model is much more developed. It treats the stored
graph as a mixed semantic artifact and repeatedly projects it for algorithms:
community detection, centrality, surprise ranking, import-cycle detection,
visualization, and export. See `graphify-semantic-model-analysis.md:878-895`.

It also documents why algorithms over mixed relation types can produce noise:
degree can find file hubs or extractor artifacts, confidence is not the same
as graph weight, cycles only make sense in directed dependency projections,
and hyperedges require explicit projection choices. See
`graphify-semantic-model-analysis.md:897-1245`.

SemanticGraph's graph-theory model is currently simple:

- directed property graph with evidence side tables;
- file-rooted containment forest;
- production Rust `references` edges overlaid on that forest;
- no hyperedges;
- no persisted centrality/community/cycle/layout projections.

See `semanticgraph-codebase-analysis.md`.

Head-to-head:

| Algorithmic topic | Graphify | SemanticGraph | Consequence |
| --- | --- | --- | --- |
| Communities | Implemented and refined with hub/low-cohesion handling | Not implemented | Defer until relation graph is richer |
| Centrality | God nodes and bridge-node heuristics | Not implemented | Add only per projection |
| Cycles | Directed import-cycle projection | Not implemented | Add after import/package edges |
| Surprise ranking | Product heuristic over structure/confidence/community | Not implemented | Treat as derived finding, not edge |
| Hyperedges | Supported as graph metadata/projection pressure | Not implemented | Use first-class tables if added |
| Projection policy | Mature but artifact-adjacent | Clean but immature | Make projection metadata durable |

Deep conclusion:

SemanticGraph should not run global graph algorithms over mixed `contains` and
`references` edges and call the results architecture. Graphify proves the value
of algorithms, but also proves they must be projection-specific.

## Query And Search

Graphify's query layer is richer as a product surface. It loads graph JSON,
uses normalized labels, IDF-like scoring, context filters, BFS/DFS traversal,
hub expansion limits, and optional query logging. See
`graphify-semantic-model-analysis.md:827-851`.

SemanticGraph's query layer is cleaner as a backend boundary. It opens SQLite
read-only, serves JSON-RPC methods, returns bounded projections, fetches
details/evidence on demand, and searches name/display/qualified/path with
escaped `LIKE` patterns. See `semanticgraph-codebase-analysis.md`.

Head-to-head:

- Graphify is better at graph-assisted exploration today.
- SemanticGraph is better at evidence-backed inspection today.
- SemanticGraph's search should eventually use its FTS table and relation-aware
  query projections.
- Graphify's local telemetry model should not be copied into durable graph
  state.

## UI And Product Surface

Graphify has a broad export/product surface: graph JSON, reports, HTML,
Obsidian, canvas, GraphML/GEXF-style exports, hooks, assistant integration,
and query tooling. Its analysis calls out that `graph.json` mixes data and
product fields such as communities, normalized labels, confidence scores,
hyperedges, and build commit. See
`graphify-semantic-model-analysis.md:768-798`.

SemanticGraph has a narrower but architecturally cleaner UI slice: Blazor
client, JSON-RPC backend, bounded projection, search, node selection, edge
selection, details, occurrences, evidence, and raw JSON. It separates durable
records, backend DTOs, diagram models, and selection state. See
`semanticgraph-codebase-analysis.md`.

Head-to-head:

- Graphify is the stronger product artifact generator.
- SemanticGraph is the stronger durable inspector.
- SemanticGraph should add richer UI controls only when the backing relations
  exist.
- SemanticGraph should keep UI/search/layout fields out of canonical node and
  edge rows unless they are explicitly versioned derived projections.

## Validation And Maturity

Graphify is mature in breadth. Its analysis cites tests and implementation
around confidence, hyperedges, semantic similarity, language extraction,
clustering, analysis, query, and lifecycle behavior.

SemanticGraph is mature in slice quality. Its analysis cites store tests,
extract tests, smoke tests, visualizer backend tests, and strict lint
expectations. See `semanticgraph-codebase-analysis.md`.

Head-to-head:

- Graphify has broader behavioral coverage.
- SemanticGraph has strong end-to-end coverage for its current Rust document
  symbol and reference routes.
- SemanticGraph's test pattern should scale with new relation families:
  fixture mapper test, persistence test, smoke count, query/detail test, and
  UI behavior check where relevant.

## Where Graphify Is Clearly Ahead

Graphify is ahead in:

- relation vocabulary breadth;
- corpus diversity beyond code;
- cache and artifact lifecycle maturity;
- query ergonomics;
- export surfaces;
- graph analysis functions;
- product-facing graph summaries;
- community/cohesion/centrality/cycle heuristics;
- hyperedge pressure and visualization;
- assistant/hook workflow integration.

These are not reasons to copy its storage model. They are evidence that a
useful semantic graph product needs more than extraction and a database.

## Where SemanticGraph Is Clearly Ahead

SemanticGraph is ahead in:

- durable SQLite source-of-truth design;
- canonical graph rows separated from proof rows;
- first-class directed edges;
- deterministic hashed canonical IDs;
- run tracking as database state;
- read-only backend projection boundary;
- typed DTOs and inspector data;
- evidence-first UI inspection;
- strict Rust error/lint style;
- smoke-tested Rust workspace route.

These are not enough to make it semantically complete. They are evidence that
the foundation is worth preserving while adding richer routes.

## Non-Negotiable Lessons

1. Do not make a generated artifact the source of truth.
   Graphify's artifact is useful, but SemanticGraph's SQLite/evidence boundary
   is the better durable foundation.

2. Do not add algorithms before relation richness.
   Graphify has strong algorithm examples, but SemanticGraph should first add
   explicit projections over its now-mixed containment/reference graph.

3. Do not treat confidence as graph weight.
   Graphify shows why uncertainty and topology are different dimensions.
   SemanticGraph already has both confidence score and weight fields.

4. Do not collapse route freshness into file freshness.
   Graphify's AST/semantic manifest split shows the pressure. SemanticGraph
   has generalized it into per-route database state for document symbols and
   references; future routes should reuse that pattern.

5. Do not overload `context` accidentally.
   Graphify shows context is useful. SemanticGraph has a context column, but
   only references currently use it meaningfully. Add policy before adding
   many relation families.

6. Do not interpret UI projection as whole-graph truth.
   Both analyses independently converge on projection discipline.

## Recommended Synthesis

The best combined architecture is:

```text
provider-backed extraction routes
  -> provider observations and source evidence
  -> canonical SQLite nodes and directed typed edges
  -> route freshness and stale closing
  -> named graph projections
  -> derived metrics and product summaries
  -> UI/export artifacts
```

Graphify contributes:

- vocabulary pressure;
- confidence semantics;
- context/subtyping examples;
- lifecycle states to account for;
- graph algorithm and projection ideas;
- export and query product expectations.

SemanticGraph contributes:

- durable schema boundary;
- evidence separation;
- directed edge storage;
- provider-neutral extraction model;
- read-only projection API;
- UI inspection model.

## Implementation Priorities

Priority 1: route freshness and stale closing.

Implemented for Rust document symbols and references. Keep this as a mandatory
pattern for every new route: explicit route status, observations, successful
route completion before stale closing, and no stale closing on failed routes.

Priority 2: the next relation-rich Rust route.

References are implemented end to end. The next best candidate is calls,
implemented with provider-backed facts, canonical edges, edge evidence,
confidence, context, persistence tests, smoke counts, and visualizer details.
Do not add three half-routes.

Priority 3: controlled relation/context vocabulary.

Use Graphify's observed vocabulary as input, but normalize it into a smaller
durable vocabulary plus context. SemanticGraph's production data should stop
having `context = None` for every route once richer facts arrive.

Priority 4: projection metadata.

Before centrality, communities, cycles, or surprise ranking, add a projection
definition model: included node kinds, relation filters, direction policy,
parallel-edge collapse policy, weight policy, confidence policy, algorithm,
parameters, source run/snapshot.

Priority 5: search and UI enrichment.

Move search toward FTS and relation-aware filtering after the graph has richer
relations. Keep UI state, layout, and search indexes out of canonical graph
truth.

## Decision Matrix For Future Work

| Proposed feature | Use Graphify as guide? | Use SemanticGraph as authority? | Notes |
| --- | --- | --- | --- |
| Confidence labels | Yes | Yes | Already aligned |
| Confidence score ranges | Yes | Partly | Need route policy |
| Relation vocabulary | Yes, as catalog | Yes, as controlled schema | Do not import all labels blindly |
| Context/subtyping | Yes | Yes | Make query-critical context structured |
| Source evidence | Partly | Yes | Preserve observations separately |
| Cache strategy | Yes, as pressure | No, not as truth | Cache is acceleration |
| Manifest/freshness | Yes, as pressure | Yes, with DB routes | Extend per-route freshness |
| Graph JSON export | Yes | No | Export from SQLite, not truth |
| Communities | Yes | Not yet | Wait for explicit relation-rich projections |
| God nodes | Yes | Not yet | Projection-specific only |
| Surprise ranking | Yes | Not yet | Derived finding with score policy |
| Hyperedges | Yes | Not yet | First-class tables if adopted |
| Query telemetry | Yes, cautiously | No canonical storage | Local/opt-in only |
| UI inspector | Partly | Yes | Keep evidence-first inspector |

## Bottom Line

Graphify is the stronger example of what a semantic graph product can do once
it has many kinds of edges, projections, summaries, and artifacts.

SemanticGraph is the stronger example of how a semantic graph should be stored
if the goal is durable, auditable, incrementally refreshable code intelligence.

The next correct move is not to make SemanticGraph look like Graphify. VS-10
proved the intended path: add one richer provider-backed relation route while
keeping the SQLite/evidence/freshness model intact. The next moves should add
another high-value provider-backed route, likely calls, and then bring in
Graphify-style projection and analysis ideas only through explicit
relation-aware projections.
