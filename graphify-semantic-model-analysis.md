# Graphify Semantic Model Analysis

Status: Research note

Date: 2026-06-14

Audited Graphify revision:
`813db192520971a4e880fab7308b9cca886a0333`
(`submodules/graphify`, reported as `v0.8.30-108-g813db19`).

## Purpose

This note is a durable analysis of Graphify's semantic modelling behavior. It is
not an ADR and does not change this repository's current direction. Its purpose
is to keep future Rust/C# semantic modelling work from rediscovering the same
Graphify findings repeatedly.

The short version:

- Graphify is strongest as a reference for graph shape, confidence tagging,
  source provenance, incremental graph artifact management, deduplication
  pitfalls, and UI/export-oriented analysis.
- Graphify's code graph is mostly syntax-first tree-sitter extraction, with
  conservative name-based cross-file resolution layered on top.
- Graphify's richer "semantic" layer is primarily LLM extraction over docs,
  papers, images, and transcripts, not code.
- For this repository, Graphify is useful as a vocabulary and product-shape
  reference, but Rust/C# symbol truth should remain `rust-analyzer` and
  Roslyn/LSP backed.

## Evidence Base

Primary files inspected:

- `submodules/graphify/docs/how-it-works.md`
- `submodules/graphify/ARCHITECTURE.md`
- `submodules/graphify/README.md`
- `submodules/graphify/graphify/extract.py`
- `submodules/graphify/graphify/detect.py`
- `submodules/graphify/graphify/cache.py`
- `submodules/graphify/graphify/querylog.py`
- `submodules/graphify/graphify/watch.py`
- `submodules/graphify/graphify/__main__.py`
- `submodules/graphify/graphify/symbol_resolution.py`
- `submodules/graphify/graphify/build.py`
- `submodules/graphify/graphify/export.py`
- `submodules/graphify/graphify/serve.py`
- `submodules/graphify/graphify/semantic_cleanup.py`
- `submodules/graphify/graphify/validate.py`
- `submodules/graphify/graphify/cargo_introspect.py`
- `submodules/graphify/graphify/skills/codex/references/update.md`
- `submodules/graphify/graphify/skills/codex/references/add-watch.md`
- `submodules/graphify/tests/test_confidence.py`
- `submodules/graphify/tests/test_hypergraph.py`
- `submodules/graphify/tests/test_semantic_similarity.py`

## Core Finding

Graphify has two distinct semantic layers that are easy to conflate:

1. Code structure extraction.
   This is local, deterministic, tree-sitter based, and runs without LLM calls.
   It extracts files, classes/types, functions/methods, imports, containment,
   some type references, some inheritance/implementation edges, and call edges.
   See `submodules/graphify/docs/how-it-works.md:7-10` and
   `submodules/graphify/ARCHITECTURE.md:60-63`.

2. Corpus semantic extraction.
   This is LLM-driven and mainly applies to docs, papers, images, and
   transcripts. Pure-code corpora skip this pass. It extracts named concepts,
   rationale, citations, semantic similarity, and sparse hyperedges. See
   `submodules/graphify/docs/how-it-works.md:10-16` and
   `submodules/graphify/graphify/skills/codex/references/extraction-spec.md:3-28`.

For this repository, that distinction matters. We should not treat Graphify's
code extractor as a semantic oracle. For Rust and C#, the equivalent facts
should be resolved by language servers where possible. Graphify's design value
is the shape of the graph and the handling of uncertain evidence, not the
authority of its syntax-based extraction.

## Graph Data Contract

Graphify's public extraction contract is intentionally simple:

- Nodes have `id`, `label`, `file_type`, `source_file`, and optional
  `source_location`.
- Edges have `source`, `target`, `relation`, `confidence`, `source_file`, and
  optional `source_location`, `confidence_score`, `weight`, and `context`.
- Hyperedges are stored as a top-level array and later as graph metadata.

Evidence:

- The architecture document defines the extraction output schema in
  `submodules/graphify/ARCHITECTURE.md:33-47`.
- Validation requires node fields and edge fields but does not enforce a
  closed relation vocabulary. See
  `submodules/graphify/graphify/validate.py:4-7` and
  `submodules/graphify/graphify/validate.py:51-57`.
- The graph format section documents NetworkX node-link JSON and hyperedges in
  `submodules/graphify/docs/how-it-works.md:83-98`.

Implication for this repo:

Graphify's JSON schema is too loose to use as the durable model. A SQLite model
should keep a closed canonical relation vocabulary, preserve raw provider
payloads separately, and allow Graphify-compatible exports as a projection.

## Node Model

Graphify does not model node kinds as a first-class typed enum. Instead, it
uses:

- `file_type` as a broad corpus category: `code`, `document`, `paper`, `image`,
  `rationale`, or `concept`.
- `label` conventions to distinguish code concepts, such as method labels like
  `.foo()` or `foo()`.
- `source_file` and `source_location` as provenance.
- Occasional `metadata` fields for extractor-specific details.
- Node ID naming conventions to imply file and symbol identity.

The validation layer allows the six broad file types above. See
`submodules/graphify/graphify/validate.py:4-7`. The semantic fragment
validator also allows those values, with payload-size and ID-shape limits. See
`submodules/graphify/graphify/semantic_cleanup.py:25-31`.

There is a notable tension: Graphify's LLM prompt allows `file_type:"rationale"`
and `file_type:"concept"` for semantic fragments, but `semantic_cleanup.py`
later treats those as invalid semantic entity types for cleanup purposes and
removes them, sometimes folding prose rationale into a `rationale` attribute on
another node. See
`submodules/graphify/graphify/skills/codex/references/extraction-spec.md:17-28`
and `submodules/graphify/graphify/semantic_cleanup.py:159-282`.

Implication for this repo:

Use explicit node kinds. `file_type` is useful as a corpus/source category, but
it is not a substitute for semantic node kinds such as `workspace`, `package`,
`file`, `namespace`, `module`, `type`, `trait`, `interface`, `impl_block`,
`function`, `method`, `field`, `property`, `enum_variant`, `parameter`, and
`local`. Rationale should likely be attached as evidence/metadata or a document
annotation, not be allowed to blur canonical code-symbol identity.

## ID Strategy

Graphify IDs are deterministic strings derived from file stems and entity
labels. The in-code helper normalizes Unicode, punctuation, and case. See
`submodules/graphify/graphify/extract.py:50-83`.

The prompt contract requires IDs of the form `{stem}_{entity}`, with only one
parent directory level, and it explicitly says these IDs must match the AST
extractor's IDs. See
`submodules/graphify/graphify/skills/codex/references/extraction-spec.md:25-28`.

Graphify has accumulated repair logic around this ID strategy:

- It remaps absolute-path-derived file node IDs to the canonical parent/stem
  form. See `submodules/graphify/graphify/extract.py:11747-11821`.
- It disambiguates colliding node IDs after per-file extraction. See
  `submodules/graphify/graphify/extract.py:6989-7049`.
- It removes LLM "ghost duplicate" nodes when an AST-origin node with the same
  basename and label exists. See `submodules/graphify/graphify/build.py:160-216`.

Implication for this repo:

Graphify's ID strategy is adequate for a portable JSON artifact, but not robust
enough for durable Rust/C# symbol identity. This repo should continue using
deterministic IDs based on workspace, language, and language-server/Roslyn
symbol keys, while retaining source ranges as evidence rather than identity.

## Relation Vocabulary

Graphify has a small nominal semantic relation set in code:

- `inherits`
- `implements`
- `mixes_in`
- `embeds`
- `references`
- `calls`
- `imports`
- `imports_from`
- `re_exports`
- `contains`
- `method`

See `submodules/graphify/graphify/extract.py:107-114`.

Actual emitted relations are broader. Observed examples include:

- Structure and ownership: `contains`, `defines`, `method`, `case_of`.
- Type and inheritance: `inherits`, `implements`, `mixes_in`, `embeds`,
  `extends`.
- Reference and dependency: `references`, `imports`, `imports_from`,
  `re_exports`, `dynamic_import`, `includes`, `exports`,
  `crate_depends_on`.
- Calls and construction: `calls`, `instantiates`.
- Framework/domain-specific edges: `uses`, `uses_config`,
  `uses_static_prop`, `references_constant`, `bound_to`, `listened_by`,
  `uses_component`, `binds_method`, `configures`, `navigates`, `triggers`,
  `reads_from`.
- LLM/concept edges: `cites`, `conceptually_related_to`,
  `shares_data_with`, `semantically_similar_to`, `rationale_for`.
- Hyperedge relations from the prompt/tests: `participate_in`, `implement`,
  `form`.

Evidence:

- Dynamic import, PHP framework, rationale, and other concrete relation literals
  appear in `submodules/graphify/graphify/extract.py:3308-3469`,
  `submodules/graphify/graphify/extract.py:3568`,
  `submodules/graphify/graphify/extract.py:3698-3904`, and
  `submodules/graphify/graphify/extract.py:4288-4308`.
- Rust emits `contains`, `method`, `inherits`, `implements`, `references`,
  `imports_from`, and `calls`. See
  `submodules/graphify/graphify/extract.py:6289-6574`.
- C# uses the generic extractor and has C#-specific inheritance/interface and
  type-reference logic. See `submodules/graphify/graphify/extract.py:1963-1968`,
  `submodules/graphify/graphify/extract.py:2474-2507`, and
  `submodules/graphify/graphify/extract.py:4049-4051`.
- Cargo introspection emits `crate_depends_on`. See
  `submodules/graphify/graphify/cargo_introspect.py:1-89`.
- Semantic similarity is first-class in tests and reporting. See
  `submodules/graphify/tests/test_semantic_similarity.py:1-185`.
- Hyperedges are tested as top-level group relationships. See
  `submodules/graphify/tests/test_hypergraph.py:1-210`.

Implication for this repo:

Do not blindly import Graphify's relation strings as the canonical vocabulary.
Instead, split the model into:

- A small closed core for durable code semantics.
- Relation contexts for language/framework-specific nuance.
- Optional extension relations for product integrations.
- A separate inferred/document-concept layer for `semantically_similar_to`,
  rationale, and conceptual links.

## Context as Subtyping

Graphify often uses `relation = "references"` plus a `context` value to carry
semantic detail. The core contexts include:

- `field`
- `parameter_type`
- `return_type`
- `generic_arg`
- `attribute`
- `value`
- `type`

See `submodules/graphify/graphify/extract.py:112-114` and
`submodules/graphify/graphify/extract.py:132-144`.

Tests assert these contexts for C, C++, C#, Java, Kotlin, PHP, Swift, and other
languages. For example, C# tests expect `parameter_type`, `return_type`,
`generic_arg`, and `field` contexts. See
`submodules/graphify/tests/test_languages.py:289-318`.

Implication for this repo:

This is a useful modelling pattern, but it should be made explicit. The durable
schema can store `relation = references` and `context = parameter_type`, or it
can promote high-value contexts into typed relations such as `parameter_type`,
`return_type`, and `field_type`. The key is to avoid burying query-critical
semantics in unstructured JSON.

## Confidence Model

Graphify uses three confidence labels:

- `EXTRACTED`: directly observed in source.
- `INFERRED`: reasonable inference.
- `AMBIGUOUS`: uncertain and worth review.

See `submodules/graphify/docs/how-it-works.md:34-49` and
`submodules/graphify/ARCHITECTURE.md:50-56`.

The LLM prompt requires every edge to have a `confidence_score`. It fixes
`EXTRACTED` at `1.0`, offers a discrete rubric for `INFERRED`, and says
`AMBIGUOUS` should be in the `0.1` to `0.3` range. See
`submodules/graphify/graphify/skills/codex/references/extraction-spec.md:13-23`.

The tests enforce the broad confidence invariants:

- `EXTRACTED` edges score `1.0`.
- `INFERRED` scores are present and between `0.0` and `1.0`.
- `AMBIGUOUS` scores are present and no more than `0.4`.

See `submodules/graphify/tests/test_confidence.py:1-120`.

Important nuance: Graphify sometimes marks syntax-observed facts as
`EXTRACTED` even when semantic target resolution is only local/name-based. It
also emits inferred cross-file calls from unique-name matching. See
`submodules/graphify/graphify/extract.py:11847-11966` and
`submodules/graphify/graphify/symbol_resolution.py:305-353`.

Implication for this repo:

Keep Graphify's confidence labels, but define "EXTRACTED" more strictly for
language-server-backed code facts:

- `EXTRACTED`: resolved by language server/Roslyn/rust-analyzer or directly
  observed from an authoritative manifest/config.
- `INFERRED`: name-only, topology-only, heuristic, or LLM-derived.
- `AMBIGUOUS`: multiple plausible targets, conflicting provider outputs, or
  unresolved language-server ambiguity.

## Code Extraction Semantics

Graphify's extractor pattern is:

1. Parse one file with tree-sitter.
2. Add a file node.
3. Add symbol nodes for declarations.
4. Add structure edges such as `contains`, `defines`, and `method`.
5. Add syntax-derived relation edges such as imports and inheritance.
6. Walk function bodies for same-file calls.
7. Save unresolved calls as `raw_calls`.
8. After all files are known, resolve some cross-file edges.

Evidence:

- Generic extractor entry point: `submodules/graphify/graphify/extract.py:2166`.
- Unresolved calls collected as `raw_calls`:
  `submodules/graphify/graphify/extract.py:3163-3317`.
- Corpus-level aggregation and cross-file resolution:
  `submodules/graphify/graphify/extract.py:11654-11990`.
- Separate deterministic symbol resolution helpers:
  `submodules/graphify/graphify/symbol_resolution.py:34-69`,
  `submodules/graphify/graphify/symbol_resolution.py:216-302`, and
  `submodules/graphify/graphify/symbol_resolution.py:305-353`.

Graphify is conservative in some places:

- Non-code nodes are excluded from call target resolution. See
  `submodules/graphify/graphify/symbol_resolution.py:34-69`.
- Member calls are skipped for cross-file raw-call resolution. See
  `submodules/graphify/graphify/symbol_resolution.py:305-353`.
- Ambiguous global labels are skipped unless import evidence uniquely
  disambiguates the target. See
  `submodules/graphify/graphify/extract.py:11895-11966`.
- Cross-language inferred calls are dropped during graph build. See
  `submodules/graphify/graphify/build.py:245-261`.

But it remains syntax-first:

- It cannot know overload resolution, trait/interface dispatch, generated code,
  conditional compilation, macro expansion, partial classes, or project-level
  semantic binding unless a special heuristic has been added.
- Some `EXTRACTED` edges mean "the syntax was observed", not "the semantic
  target was resolved by a compiler-grade engine".

Implication for this repo:

Graphify's extraction pipeline is useful as a staging architecture, but this
repo's code semantic facts should be provider-backed. Use Graphify-like
`raw_calls` only as an intermediate/evidence artifact when the provider cannot
answer a fact directly.

## Rust Findings

Graphify has a custom Rust tree-sitter extractor. It extracts:

- `function_item` as file-contained functions.
- `struct_item`, `enum_item`, and `trait_item` as code nodes.
- Trait bounds as `inherits` or `references` with `generic_arg` context.
- Struct field type references.
- `impl_item` methods as `method` edges.
- Trait impls as `implements`.
- `use_declaration` as `imports_from`.
- Same-file calls as `calls`.
- Some unresolved calls as `raw_calls`.

See `submodules/graphify/graphify/extract.py:6289-6574`.

Graphify deliberately avoids cross-file resolution for common Rust trait/stdlib
method names and for `Type::method()` scoped calls because bare last-segment
matching creates spurious edges across crate boundaries. See
`submodules/graphify/graphify/extract.py:6289-6298` and
`submodules/graphify/graphify/extract.py:6528-6558`.

Graphify also has a separate Cargo manifest introspector that emits workspace
crate dependency edges as `crate_depends_on`. See
`submodules/graphify/graphify/cargo_introspect.py:1-89`.

Implication for this repo:

The Rust extractor is evidence that certain edge categories are useful:
`contains`, `method`, `implements`, `inherits`, `references` with type contexts,
`imports_from`, `calls`, and crate dependencies. It is not sufficient authority
for Rust semantics. `rust-analyzer` should provide document symbols,
definitions, references, implementations, type information, call hierarchy, and
macro-aware project context where available.

## C# Findings

Graphify's C# extractor delegates to the generic tree-sitter path. See
`submodules/graphify/graphify/extract.py:4049-4051`.

It has C#-specific additions inside the generic walker:

- A pre-scan of local interfaces. See
  `submodules/graphify/graphify/extract.py:543-575`.
- Base-list handling that classifies a base as `implements` if it is a declared
  interface or follows an `IName` convention, otherwise `inherits`. See
  `submodules/graphify/graphify/extract.py:2474-2507`.
- Type-reference handling for fields, parameters, return types, generic
  arguments, and attributes. See
  `submodules/graphify/graphify/extract.py:2720-3120`.
- Call extraction from invocation expressions by callee name. See
  `submodules/graphify/graphify/extract.py:3239-3248`.

Implication for this repo:

Graphify identifies the right categories, but not with Roslyn-grade precision.
For C#, `csharp-language-server` and Roslyn-backed flows should decide
definition, references, type hierarchy, implementation, and call edges. The
Graphify model is a useful checklist for categories, especially
`parameter_type`, `return_type`, `generic_arg`, `field`, `inherits`, and
`implements`.

## Rationale and Comments

Graphify treats rationale as graph-relevant information. The README advertises
extracting "why" information from comments, docstrings, and design documents.
See `submodules/graphify/README.md:231-233`.

The Python extractor has a post-pass for docstrings and rationale comments such
as `NOTE`, `IMPORTANT`, `HACK`, `WHY`, `RATIONALE`, `TODO`, and `FIXME`. It adds
rationale nodes and `rationale_for` edges. See
`submodules/graphify/graphify/extract.py:3517-3569`.

The semantic cleanup layer later converts sentence-like rationale nodes into
attributes on target nodes, but only along `rationale_for` edges. See
`submodules/graphify/graphify/semantic_cleanup.py:159-282`.

Implication for this repo:

Rationale is valuable, but should be modelled separately from canonical code
symbols. Good options:

- Store rationale as source occurrences or annotations linked to nodes.
- Use `edge_evidence` for rationale that justifies inferred edges.
- Keep document-derived rationale in a document/concept layer unless it refers
  to a resolved code symbol.

## Hyperedges

Graphify supports hyperedges as top-level graph metadata, not as ordinary edge
records. The LLM prompt allows up to three hyperedges per chunk for group
relationships not well captured by pairwise edges. See
`submodules/graphify/graphify/skills/codex/references/extraction-spec.md:20-28`.

Build/export logic stores hyperedges on `G.graph["hyperedges"]`. See
`submodules/graphify/graphify/build.py:280-282` and
`submodules/graphify/graphify/export.py:463-471`.

Tests assert hyperedges survive build and JSON export, and appear in reports.
See `submodules/graphify/tests/test_hypergraph.py:1-210`.

Semantic cleanup filters hyperedges so they only reference surviving nodes and
drops hyperedges with fewer than two surviving members. See
`submodules/graphify/graphify/semantic_cleanup.py:261-282`.

Implication for this repo:

Hyperedges are useful for high-level design flows, protocols, feature slices,
or "these symbols participate in one concept" groupings. They should remain
optional. If implemented, they should be first-class SQLite tables rather than
opaque graph metadata, because membership needs provenance, source evidence,
and incremental invalidation.

## Directionality and NetworkX

Graphify defaults to undirected NetworkX graphs for backward compatibility but
many relations are directional. It preserves true endpoints with `_src` and
`_tgt` attributes during build/export. See
`submodules/graphify/graphify/build.py:107-111`,
`submodules/graphify/graphify/build.py:216-280`, and
`submodules/graphify/graphify/export.py:521-531`.

There is dedicated defensive code around direction loss:

- Build comments note that `nx.Graph` can silently flip directional edges when
  reading through a node-link round trip. See
  `submodules/graphify/graphify/build.py:396-402`.
- Export restores original direction from `_src` and `_tgt`. See
  `submodules/graphify/graphify/export.py:521-531`.
- Tests cover direction-preservation regressions for `calls`. See
  `submodules/graphify/tests/test_build.py:158-274`.

Implication for this repo:

This strongly supports the current decision to model directed edges as
first-class rows in SQLite. Direction should not be an attribute on an
undirected storage primitive.

## Merge, Deduplication, and Evidence Loss

Graphify's graph build and merge behavior is artifact-oriented:

- Nodes are added by ID; later additions overwrite earlier attributes. See
  `submodules/graphify/graphify/build.py:3-16` and
  `submodules/graphify/graphify/build.py:303-306`.
- Dedup rewrites edges to surviving node IDs. See
  `submodules/graphify/graphify/dedup.py:145-176` and
  `submodules/graphify/graphify/dedup.py:344-352`.
- `build_merge` loads existing `graph.json`, merges chunks, and prevents
  accidental graph shrinkage unless explicit pruning is requested. See
  `submodules/graphify/graphify/build.py:378-476`.
- Ghost duplicate cleanup removes non-AST semantic nodes that appear to
  duplicate AST nodes. See `submodules/graphify/graphify/build.py:160-216`.

This is pragmatic for a generated artifact. It is risky as a durable source of
truth because raw observations can be overwritten, collapsed, or removed.

Implication for this repo:

Keep the ADR's separation between canonical nodes/edges and raw evidence. A
dedup decision should produce a canonical mapping while preserving every
occurrence and provider observation that led to it.

## Analysis and Product Semantics

Graphify's downstream analysis treats graph structure as the semantic substrate:

- Community detection uses topology; semantic similarity edges influence
  topology directly. See `submodules/graphify/docs/how-it-works.md:26-30`.
- `semantically_similar_to` receives special scoring/report treatment. See
  `submodules/graphify/tests/test_semantic_similarity.py:1-185`.
- Structural edges such as `imports`, `imports_from`, `contains`, and `method`
  are often filtered out of "surprising connection" analysis. See
  `submodules/graphify/graphify/analyze.py:287-288` and
  `submodules/graphify/graphify/analyze.py:379-383`.
- Cross-language inferred `calls`/`uses` edges are suppressed in surprise
  scoring because they are likely extraction artifacts. See
  `submodules/graphify/graphify/analyze.py:209-224`.

Implication for this repo:

The semantic model should classify relations by analytical role, not only by
edge label:

- Structural containment edges.
- Dependency/import edges.
- Executable behavior edges.
- Type-system edges.
- Evidence/provenance edges.
- Inferred conceptual edges.
- Product/reporting-only or projection edges.

This classification will make projections and UI filtering more stable than a
flat relation string list.

## Tracked Data And Lifecycle

Graphify's tracked data model is deeper than its node-link JSON implies. It
does not only track "the graph." It tracks a corpus scan, normalized input
sidecars, extraction fragments, cache keys, merge/prune state, materialized
graph JSON, analysis projections, report artifacts, serving/search state,
hook/watch state, and local telemetry.

The important insight for this repository is separation of lifecycles.
Graphify places many lifecycles under `graphify-out/` for convenience. That is
reasonable for a portable generated artifact, but it is exactly what a durable
SQLite model should avoid. Canonical facts, raw evidence, refresh ledgers,
caches, UI projections, and local query logs should not have the same status.

### Output Directory As Mixed State

Graphify explicitly presents `graphify-out/` as the team artifact directory. It
contains `graph.html`, `GRAPH_REPORT.md`, and `graph.json`; the README says
`graphify-out/` is meant to be committed, while `cost.json` is local-only and
`cache/` is optional. It also calls out `manifest.json` as portable because
keys are stored as relative paths and re-anchored on load. See
`submodules/graphify/README.md:37-40`,
`submodules/graphify/README.md:325-333`.

That creates this data taxonomy:

| Data class | Graphify examples | Lifecycle | Meaning for this repo |
| --- | --- | --- | --- |
| Canonical-ish graph artifact | `graphify-out/graph.json` | Committed/generated, merge-driver aware | Useful export shape, but not a source-of-truth schema |
| Human report | `GRAPH_REPORT.md`, `graph.html`, Obsidian notes, canvas exports | Regenerated projection | UI/report cache, not graph truth |
| Corpus ledger | `manifest.json` | Committed refresh baseline | Equivalent should be explicit source-file/run tables |
| Normalized input | `graphify-out/converted/` sidecars | Derived from source files | Needs source-to-derived provenance if used |
| Extraction fragments | `.graphify_detect.json`, `.graphify_incremental.json`, `.graphify_extract.json`, `.graphify_old.json` | Transient pipeline state | Should become structured extraction-run records, not hidden files |
| Performance cache | `cache/ast/v...`, `cache/semantic/`, `cache/stat-index.json` | Disposable, sometimes portable | Optional acceleration only |
| Analysis sidecars | `.graphify_analysis.json`, `.graphify_labels.json`, `.graphify_semantic_marker` | Derived analytics/control metadata | Projection metadata with explicit versioning |
| Watch/hook state | `needs_update`, `.graphify_python`, hook merge driver config | Runtime coordination | Scheduler/control state, separate from graph data |
| Local telemetry | query JSONL, `cost.json` | Local/private | Never canonical graph state |

### Corpus Discovery Ledger

`detect.py` is the first tracked-data layer. It classifies files into broad
corpus buckets, records skipped sensitive files, converts some input files to
sidecars, counts words, and returns `needs_graph`, `total_words`,
`skipped_sensitive`, `graphifyignore_patterns`, and `scan_root`. See
`submodules/graphify/graphify/detect.py:997-1149`.

The manifest is a refresh ledger, not a graph. `_MANIFEST_PATH` is
`graphify-out/manifest.json`; load/save helpers store portable relative keys
where possible. The manifest entry tracks `mtime`, `ast_hash`, and
`semantic_hash`. `save_manifest(kind="ast")` stamps `ast_hash` and preserves
`semantic_hash` only when the content hash still matches; `kind="semantic"`
stamps `semantic_hash` and preserves `ast_hash`. See
`submodules/graphify/graphify/detect.py:27`,
`submodules/graphify/graphify/detect.py:1222-1322`, and
`submodules/graphify/graphify/detect.py:1337-1391`.

The incremental detector splits the corpus into `new_files`,
`unchanged_files`, and `deleted_files`. Missing `semantic_hash` means a file is
semantically stale, even if the AST path has seen it. See
`submodules/graphify/graphify/detect.py:1325-1424`.

Deep implication:

Graphify effectively has two freshness clocks per file: one for deterministic
AST extraction and one for semantic extraction. That is the right shape. This
repo should model those clocks explicitly as extraction-route freshness rather
than a single "file hash changed" flag. A Rust file may be fresh for document
symbols but stale for call hierarchy; a C# file may be fresh for references but
stale for type hierarchy. A single source-file row is not enough.

### Converted Inputs Are Derived Corpus State

Graphify writes converted office and Google Workspace files into
`graphify-out/converted/`, skips that directory during future scans, and
records conversion failures as skipped-sensitive-style warnings. See
`submodules/graphify/graphify/detect.py:1070-1117` and
`submodules/graphify/README.md:267`.

`convert_office_file` writes a Markdown sidecar and avoids rewriting existing
sidecars to prevent modification-time churn. The sidecar name is deterministic
from the resolved source path, and the content carries a "converted from"
comment. See `submodules/graphify/graphify/detect.py:599-631`.

Deep implication:

Converted inputs are neither raw evidence nor canonical graph facts. They are
derived inputs to an extractor. If this repo later ingests design documents,
the storage model should distinguish:

- original source asset,
- normalized text representation,
- extraction run over that representation,
- canonical graph facts produced from the run,
- evidence spans pointing back to the normalized representation and, where
  possible, the original asset.

For Rust/C# code, the analogous distinction is between source text,
language-server semantic responses, and canonical graph facts. The language
server response is an observation artifact. It should not be flattened directly
into a canonical edge without preserving route, version, and source range.

### Cache State Is Not Evidence

Graphify's cache layer has three different meanings:

- A stat index at `graphify-out/cache/stat-index.json` maps absolute paths to
  size, nanosecond mtime, and content hash for fast hash reuse.
- AST cache entries are versioned by Graphify package version because extractor
  code changes can make old AST output invalid.
- Semantic cache entries are deliberately unversioned to avoid rebilling LLM
  extraction for unchanged files.

See `submodules/graphify/graphify/cache.py:15-24`,
`submodules/graphify/graphify/cache.py:85-99`, and
`submodules/graphify/graphify/cache.py:271-284`.

The cache key is a SHA256 over file contents plus path relative to the root.
For Markdown, YAML frontmatter is stripped before hashing so metadata-only
changes do not invalidate semantic extraction. Cached `source_file` fields are
rewritten to relative paths on disk and re-anchored to absolute paths on load.
See `submodules/graphify/graphify/cache.py:155-194`,
`submodules/graphify/graphify/cache.py:207-266`, and
`submodules/graphify/graphify/cache.py:292-351`.

Semantic cache checking groups cached nodes, edges, and hyperedges by
`source_file`, returning cached fragments plus uncached paths; saving semantic
cache writes one cache entry per source file. See
`submodules/graphify/graphify/cache.py:409-475`.

Deep implication:

This is a performance cache, not a durable evidence store. The cache preserves
fragments, but it is intentionally optimized for cost avoidance and portability
rather than auditability. In particular, unversioned semantic cache entries
trade correctness traceability for LLM cost control.

For this repo, cache rows should be disposable and versioned by extraction
route, tool version, request shape, and relevant server/project configuration.
Durable evidence should live in evidence tables, not in a cache blob. If a
semantic response is expensive enough to keep, it is not just "cache"; it is a
provider observation with provenance.

### Extraction Fragments And Token Accounting

Graphify's CLI creates separate AST and semantic result dictionaries, then
merges nodes, edges, hyperedges, and token counts. Semantic extraction checks
the semantic cache first, saves fresh semantic fragments per source file, and
counts `input_tokens` and `output_tokens`. See
`submodules/graphify/graphify/__main__.py:4245-4353` and
`submodules/graphify/graphify/__main__.py:4387-4391`.

It also creates a manifest-safe file set: semantic files only receive a
semantic manifest stamp if cached or fresh output produced a `source_file`
entry. Failed semantic chunks remain stale so the next incremental run
re-queues them. See `submodules/graphify/graphify/__main__.py:4391-4406` and
`submodules/graphify/graphify/__main__.py:4523-4556`.

The agent-oriented update flow materializes transient files:
`.graphify_incremental.json`, `.graphify_detect.json`,
`.graphify_extract.json`, and `.graphify_old.json`. It merges new extraction
with existing `graph.json`, prunes deleted and changed source files, and writes
the merged extraction back for later steps. See
`submodules/graphify/graphify/skills/codex/references/update.md:1-180`.

Deep implication:

Graphify has an implicit extraction-run model implemented through files. This
repo should make that model explicit:

- one `extraction_runs` row per run,
- one `extraction_route_runs` row per route such as document symbols,
  references, call hierarchy, type hierarchy, semantic tokens, or document LLM,
- per-route status, warnings, and provider/tool versions,
- per-source-file route freshness,
- token/cost/accounting only for routes that incur such cost.

That prevents "run succeeded" from hiding partial semantic failures, route
skips, or cache-only reuse.

### Merge, Prune, And Shrinkage

Graphify's merge path is grow-first. `build_merge` loads existing `graph.json`,
merges new chunks, and "never replaces" except when `prune_sources` is passed.
It explicitly supports pruning deleted-file nodes and skips shrinkage checks
when deduplication or pruning is active. See
`submodules/graphify/graphify/build.py:378-473`.

The update reference prunes both deleted files and changed files before
inserting fresh extraction for those files. That avoids reconciling old and new
versions of same-file nodes. See
`submodules/graphify/graphify/skills/codex/references/update.md:90-132`.

`export.to_json` also protects the existing artifact by refusing to silently
overwrite a larger graph with a smaller one unless forced. During export it
adds `community`, `norm_label`, default `confidence_score`, top-level
`hyperedges`, and `built_at_commit`, and it restores true direction from
internal `_src`/`_tgt` markers. See
`submodules/graphify/graphify/export.py:484-534`.

Deep implication:

Graphify's graph artifact is not event-sourced. It is a materialized snapshot
with overwrite, repair, dedup, and prune behavior. That is appropriate for
`graph.json`, but it would be unsafe as this repo's only durable record.

This repo's SQLite design should preserve the ADR split:

- canonical nodes and edges represent current best graph state,
- occurrences and edge evidence preserve every source observation,
- extraction runs explain when and how each observation entered the store,
- soft deletion records disappearance without erasing history,
- snapshot exports are rebuildable products of the database.

### Materialized Graph JSON Mixes Fact And Projection

Graphify writes projection fields directly into `graph.json`: communities,
normalized labels for search, confidence scores, hyperedges, and build commit.
It also writes analysis sidecars such as `.graphify_analysis.json` with
communities, cohesion, god nodes, surprises, and token counts. See
`submodules/graphify/graphify/export.py:510-534` and
`submodules/graphify/graphify/__main__.py:4494-4523`.

Other exports reinterpret the same graph for different consumers:

- HTML restores direction from `_src`/`_tgt`, colors by community, styles by
  confidence, and can aggregate large graphs to community nodes.
- Obsidian writes one note per node plus community overview notes, frontmatter,
  tags, connection lists, graph color config, and bridge-node summaries.
- Canvas and GraphML exports preserve or drop different attributes; GraphML
  explicitly drops internal `_origin`, `_src`, and `_tgt` markers as runtime
  details.

See `submodules/graphify/graphify/export.py:631-815`,
`submodules/graphify/graphify/export.py:839-1089`,
`submodules/graphify/graphify/export.py:1092-1248`, and
`submodules/graphify/graphify/export.py:1419-1434`.

Deep implication:

`graph.json` is both data and product. For this repo, durable storage should
not put UI/search fields like `norm_label`, community assignments, layout
metadata, confidence display scores, or report-only rankings into canonical
node/edge rows unless they are explicitly versioned derived facts. They belong
in projection tables or export artifacts.

### Watch State And Hook State Are Refresh Control

Graphify's watcher treats code changes differently from docs, papers, and
images. Code-only changes can rerun AST extraction and update `graph.json` and
`GRAPH_REPORT.md` without LLM work. Doc/paper/image changes write a
`graphify-out/needs_update` flag and require a later semantic update. See
`submodules/graphify/graphify/skills/codex/references/add-watch.md:42-56` and
`submodules/graphify/graphify/watch.py:776-810`.

The watcher clears stale `needs_update` flags after successful code-graph
updates, stamps `built_at_commit`, writes core outputs, and avoids rewriting
outputs when no code-graph changes are detected. See
`submodules/graphify/graphify/watch.py:612-764`.

The README also documents hook installation: post-commit auto-rebuilds and a
git merge driver union-merges `graph.json` so parallel commits do not leave
conflict markers. See `submodules/graphify/README.md:336-338` and
`submodules/graphify/README.md:475-476`.

Deep implication:

Refresh control is first-class tracked state. In this repo, route-specific
pending work should be represented as database state or a known job queue, not
as an implicit file flag. For example, "C# outgoing call hierarchy unavailable"
and "Rust call hierarchy stale for file X" are not the same state and should
not collapse into one `needs_update` bit.

### Query State And Telemetry

Graphify's server loads `graph.json`, forces the loaded data to directed, and
builds query-time helpers such as an in-memory IDF cache on the graph object.
Search scoring uses normalized labels, source-file matches, exact/prefix
bonuses, and query-context filters; BFS avoids expanding through high-degree
hubs above a percentile/floor threshold. See
`submodules/graphify/graphify/serve.py:14-35`,
`submodules/graphify/graphify/serve.py:82-194`, and
`submodules/graphify/graphify/serve.py:275-349`.

Query logging is append-only JSONL, fail-silent, and local by default. It logs
timestamp, question, corpus, node count, result size, duration, and optional
extra fields. Full responses are not stored unless
`GRAPHIFY_QUERY_LOG_RESPONSES=1`; logging can be disabled or redirected with
environment variables. See `submodules/graphify/graphify/querylog.py:1-65` and
`submodules/graphify/README.md:420-435`.

Deep implication:

Query telemetry is not graph evidence. It is product analytics and potentially
sensitive user-behavior data. This repo should keep any future query logs local
or opt-in, separate from the durable semantic store. Query-derived ranking
caches should be invalidated by graph/projection version, not treated as graph
facts.

### Durable Model Lesson

The tracked-data lesson from Graphify is not "commit `graph.json`." It is that
a useful semantic graph system needs explicit data classes:

- `source_files`: path, language/corpus kind, content hash, mtime, deletion
  state, and workspace root.
- `normalized_inputs`: optional derived text/assets with source-to-derived
  provenance.
- `extraction_runs`: run id, route, tool/server version, workspace snapshot,
  status, warnings, and cost/timing fields.
- `provider_observations`: raw LSP/Roslyn/rust-analyzer/LLM facts with request
  shape and response provenance.
- `occurrences`: source proof for nodes.
- `edge_evidence`: source proof for edges, including route and confidence.
- `nodes` and `edges`: current canonical graph state.
- `derived_projections`: community assignments, centrality, layouts, search
  indexes, graph reports, CSV snapshots, and visualizer projections.
- `local_telemetry`: query logs and interaction metrics, kept out of canonical
  graph storage.

Graphify's practical strength is that it already exposes the pressure for all
of these classes. Its weakness, for this repo's purpose, is that many of them
share one artifact boundary. SQLite should make those boundaries explicit.

## Graph Theory Model

Graphify's graph-theory model is practical rather than formal. It behaves like
a heterogeneous attributed property graph that is later projected into simpler
graphs for specific algorithms.

The important distinction:

- The stored graph is a mixed semantic artifact: files, symbols, document
  concepts, rationale, and media-derived nodes can coexist.
- The analysis graph is usually a projection: community detection,
  centrality, surprise ranking, import-cycle detection, visualization, and
  export each read only part of the stored semantics or reinterpret it.

This is the deepest Graphify lesson for this repository: graph algorithms are
not neutral over a mixed semantic graph. Relation families, node kinds,
confidence, and provenance determine whether an algorithm produces insight or
noise.

### Property Graph, Not Pure Code Graph

Graphify nodes and edges carry attributes rather than strong schema-level
types. Nodes have labels, file types, source files, communities, provenance
fields, and sometimes metadata. Edges have relation strings, confidence,
confidence scores, weights, source files, contexts, and internal direction
markers. See the graph format in
`submodules/graphify/docs/how-it-works.md:83-98` and export-time community and
confidence handling in `submodules/graphify/graphify/export.py:519-531`.

Graph-theory consequence:

A single graph contains several graphs at once:

- A containment tree or forest.
- A dependency graph.
- A call graph.
- A type/reference graph.
- A document/concept graph.
- A similarity graph.
- An evidence/provenance graph.

Running degree, centrality, clustering, or shortest-path style analysis across
all of these relation types at once gives relation-count topology, not
necessarily code architecture. Graphify compensates with filters and heuristics
in later analysis. This repo should make those projections explicit instead of
relying on ad hoc filtering at every call site.

### Simple Graph, Directed Graph, And Multigraph Pressure

Graphify currently defaults to `nx.Graph` for backward compatibility, even
though many relations are directional. It optionally supports directed build
paths, but the default undirected storage requires `_src` and `_tgt` edge
attributes to recover true direction. See
`submodules/graphify/graphify/build.py:107-111`,
`submodules/graphify/graphify/build.py:262-280`, and
`submodules/graphify/graphify/export.py:521-531`.

That creates two graph-theory problems:

- Directional semantics are not part of the graph primitive, so algorithms that
  inspect the graph structure see an undirected edge unless they explicitly
  consult `_src` and `_tgt`.
- Parallel semantic facts between the same node pair compete for storage in a
  simple graph unless relation/context/key handling preserves them.

Graphify has a dedicated future-facing MultiDiGraph capability probe. It checks
keyed parallel directed edges, NetworkX node-link round trips, duplicate-key
overwrite semantics, reserved-key protection, two-tuple removal behavior, and
conversion to undirected multigraphs. See
`submodules/graphify/graphify/multigraph_compat.py:1-220`.

Implication for this repo:

The durable graph should be a directed typed multigraph at the data-model
level, even if individual projections collapse it to undirected simple graphs.
SQLite `edges` should preserve relation, context, direction, and evidence. A
community-detection projection can intentionally collapse parallel edges into a
weighted undirected graph, but that should be a named derived graph, not the
canonical representation.

### Weights, Confidence, And Algorithmic Meaning

Graphify stores both `weight` and `confidence_score`, but they are not the same
thing.

- `confidence_score` expresses epistemic certainty: how much Graphify trusts
  the fact.
- `weight` expresses graph strength or visualization/analysis weight.
- `confidence` labels drive report styling and surprise ranking.

The export path defaults missing confidence scores by label. See
`submodules/graphify/graphify/export.py:459-531`. The confidence tests assert
round-trip presence and broad score ranges. See
`submodules/graphify/tests/test_confidence.py:1-120`.

Graphify's surprise score uses confidence directly as a ranking signal:
ambiguous and inferred edges are more noteworthy than extracted edges. See
`submodules/graphify/graphify/analyze.py:194-258`.

Graph-theory consequence:

Confidence is not always a good graph weight. A low-confidence edge may be
analytically important because it is uncertain and surprising, but it should
not necessarily bind communities strongly. Conversely, a high-confidence
`contains` edge can dominate topology while adding little architectural
insight.

Implication for this repo:

Do not collapse confidence and graph weight into one field. Store both:

- `confidence` / `confidence_score` for epistemic status.
- `weight` or projection-specific weights for algorithms.
- A named projection policy that decides which relation families contribute to
  topology, and how strongly.

### Community Detection

Graphify uses community detection to find dense topical/structural regions. It
prefers Leiden through `graspologic`, falls back to NetworkX Louvain, and
passes a resolution parameter where supported. See
`submodules/graphify/graphify/cluster.py:1-65`.

Before partitioning, Graphify creates a stable graph with sorted nodes and
sorted edge rows to reduce nondeterminism. See
`submodules/graphify/graphify/cluster.py:26-39`.

The `cluster()` function converts directed graphs to undirected graphs because
Leiden/Louvain require undirected input in this path. It also handles isolates,
optionally excludes high-degree hubs before partitioning, reattaches excluded
hubs by majority-vote neighbor community, splits oversized communities, and
re-splits large low-cohesion communities. See
`submodules/graphify/graphify/cluster.py:73-194`.

Community IDs are then sorted deterministically by size with a lexical
tiebreak, and there is a separate helper for remapping new community IDs to
previous IDs by overlap. See
`submodules/graphify/graphify/cluster.py:195-260`.

Graph-theory consequence:

Graphify treats communities as derived analysis state, not source truth. It
also acknowledges that high-degree hubs and low-cohesion mega-communities can
distort modularity-based clustering. That is why it has hub exclusion,
community splitting, cohesion checks, deterministic ordering, and previous-ID
remapping.

Implication for this repo:

Persist community results as graph snapshots with algorithm name, parameters,
input relation filters, graph projection type, source run, and cohesion
metrics. Do not store `community_id` as an inherent node property without
snapshot context. A node can belong to different communities in a call graph,
import graph, type graph, and conceptual graph.

### Cohesion

Graphify defines community cohesion as the ratio of actual intra-community
edges to the maximum possible edges. See
`submodules/graphify/graphify/cluster.py:230-241`.

It uses low cohesion to split large communities during clustering and to
generate suggested questions about whether a community should be split into
more focused modules. See `submodules/graphify/graphify/cluster.py:178-188`
and `submodules/graphify/graphify/analyze.py:520-527`.

Graph-theory consequence:

Cohesion is a density measure. It is useful for spotting loose buckets, but it
is sensitive to graph projection and node granularity. A community of files
will have different density from a community of methods, and containment edges
can inflate density without proving architectural cohesion.

Implication for this repo:

Record node granularity and relation filters with any cohesion metric. A
cohesion score without projection metadata is not durable evidence.

### God Nodes And Degree Centrality

Graphify's "god nodes" are degree-central nodes after filtering. The function
computes degree, sorts descending, and excludes file-level hubs, synthetic
method stubs, concept nodes, JSON key noise, and common builtin/mock names. See
`submodules/graphify/graphify/analyze.py:1-117`.

Graph-theory consequence:

Degree centrality is easy to compute and easy to misread. In a heterogeneous
semantic graph, high degree may mean:

- A real architectural hub.
- A file node collecting `contains` and import edges.
- A common type such as `String` or `Path`.
- A framework concept.
- A generated artifact.
- An extractor bug or name-resolution artifact.

Graphify's filters are evidence of this risk.

Implication for this repo:

Centrality should be projection-specific. Useful rankings may include:

- Top callable by incoming calls.
- Top type by references.
- Top module by import dependents.
- Top bridge symbol by betweenness in a call/import projection.
- Top uncertain node by count of `INFERRED` or `AMBIGUOUS` adjacent edges.

One global degree ranking over all relation types should be treated as a broad
exploration hint, not a semantic fact.

### Betweenness And Bridge Semantics

Graphify uses edge betweenness centrality as a fallback for single-source
surprising connections when community information is unavailable. See
`submodules/graphify/graphify/analyze.py:331-363`.

It also uses node betweenness centrality to suggest bridge-node questions. For
large graphs, it samples up to 100 nodes with a fixed seed. See
`submodules/graphify/graphify/analyze.py:448-470`.

Graph-theory consequence:

Betweenness identifies nodes or edges that lie on many shortest paths. In code
graphs, that can reveal cross-cutting abstractions, adapters, central modules,
or accidental bottlenecks. But shortest paths over mixed relation types are
hard to interpret: a path that alternates `contains`, `references`, and
`semantically_similar_to` is not the same kind of explanation as a pure call
path.

Implication for this repo:

Betweenness should run on typed projections:

- Call graph betweenness for execution bridges.
- Import graph betweenness for package/module dependency bridges.
- Type-reference graph betweenness for API/type coupling.
- Concept graph betweenness for documentation/idea bridges.

The UI should label which projection produced the bridge result.

### Surprise Ranking Is A Domain Heuristic

Graphify's "surprising connections" are not a pure graph-theory metric. They
combine graph structure with domain heuristics:

- Confidence rank: `AMBIGUOUS` and `INFERRED` are more notable.
- Cross file-type edges are more surprising.
- Cross top-level directory edges are more surprising.
- Cross-community edges are more surprising.
- `semantically_similar_to` receives a multiplier.
- Peripheral-to-hub edges receive a bonus.
- Structural edges such as `imports`, `imports_from`, `contains`, and `method`
  are filtered out.

See `submodules/graphify/graphify/analyze.py:194-328`.

Graphify also suppresses structural bonuses for inferred `calls` and `uses`
edges that cross language boundaries or connect code to docs, because these are
likely resolver pollution. See `submodules/graphify/graphify/analyze.py:209-224`.

Graph-theory consequence:

The most product-useful graph results are often hybrid scores, not textbook
metrics. Graphify's surprise ranking is a projection plus a scoring policy.

Implication for this repo:

Model derived findings as named analyses with inputs and scoring policy, not as
canonical graph edges. For example, a "surprising connection" should reference
the underlying edge IDs, projection, score components, and extraction run.

### Hypergraphs

Graphify supports hyperedges, but stores them outside the NetworkX edge set as
`G.graph["hyperedges"]`. See `submodules/graphify/graphify/build.py:280-282`
and `submodules/graphify/graphify/export.py:463-471`.

The HTML export renders hyperedges as shaded regions, and the aggregated export
can remap hyperedge members from node IDs to community IDs. See
`submodules/graphify/graphify/export.py:216-228` and
`submodules/graphify/graphify/export.py:671-694`.

Graph-theory consequence:

Hyperedges are not part of Graphify's normal NetworkX topology. They survive
round trips and visualization, but community detection, centrality, and shortest
path style algorithms do not automatically account for them unless a projection
turns them into ordinary edges or incidence nodes.

There are three common projection choices:

- Clique expansion: connect every pair of members.
- Star expansion: create a group node and connect each member to it.
- Incidence graph: maintain separate hyperedge nodes and membership edges.

Each has different graph-theory behavior. Clique expansion inflates density.
Star expansion creates hubs. Incidence graphs preserve structure but require
algorithms that understand the two-mode graph.

Implication for this repo:

If hyperedges become part of semantic modelling, store them as first-class
tables and define explicit projections for analysis. Do not silently turn them
into pairwise edges in the canonical graph.

### Directed Dependency Cycles

Graphify has a specialized import-cycle analysis that projects the full graph
into a directed file-level graph. It keeps only `imports_from` and `re_exports`
edges, resolves endpoints to source files, then runs bounded simple-cycle
enumeration. See `submodules/graphify/graphify/analyze.py:636-720`.

Tests confirm that this projection detects two-node cycles, three-node cycles,
self loops, respects maximum cycle length, handles undirected graph input by
using edge `source_file`, and ignores non-import relations. See
`submodules/graphify/tests/test_analyze.py:615-722`.

Graph-theory consequence:

Cycle analysis only makes sense on a directed dependency projection. Running
cycle detection over the whole mixed graph would produce meaningless cycles
through containment, calls, references, and conceptual edges.

Implication for this repo:

Define separate directed projections:

- File import graph.
- Package/crate/project dependency graph.
- Callable call graph.
- Type inheritance/implementation graph.

Each projection should have its own valid cycle semantics. An import cycle is a
problem; a call cycle might be recursion; an inheritance cycle should be
impossible if the provider is correct; a conceptual cycle is usually harmless.

### Projection Discipline

Graphify repeatedly demonstrates that the canonical graph and algorithm graph
are different things:

- Clustering intentionally undirects the graph.
- Surprise ranking filters structural edges.
- God-node ranking filters file and noise nodes.
- Import-cycle detection builds a directed file graph.
- Hyperedges are visual/group metadata unless projected.
- Multigraph support is being probed separately from default simple graph mode.

Implication for this repo:

The durable schema should support named graph projections as first-class
derived artifacts. A projection should record:

- Source extraction run or snapshot.
- Included node kinds.
- Included relation kinds.
- Direction policy.
- Parallel-edge collapse policy.
- Weight policy.
- Confidence policy.
- Algorithm name and parameters.
- Output metrics and generated-at timestamp.

This turns "the graph says X" into "projection P, built from run R with policy
Q, produced metric X." That is the difference between durable semantic
modelling and an attractive but ambiguous graph artifact.

## What To Borrow

Borrow these ideas directly:

- Confidence labels: `EXTRACTED`, `INFERRED`, `AMBIGUOUS`.
- Confidence scores, with fixed `1.0` for truly extracted facts.
- Separate source provenance for every node and edge.
- Directed edge semantics for relations like `calls`, `imports`,
  `implements`, `inherits`, and `rationale_for`.
- A relation/context split for type-reference detail.
- Optional hyperedges for sparse group relationships.
- Deterministic export artifacts.
- Conservative handling of ambiguous name-based resolution.
- Product-level projections such as god nodes, surprising connections, and
  community-aware exploration.

## What To Avoid

Do not borrow these as durable design choices:

- NetworkX node-link JSON as source of truth.
- Undirected graph storage with direction side-channel attributes.
- File-stem plus label IDs as canonical identity.
- Open-ended relation strings with no typed vocabulary.
- Treating LLM-derived code claims as equivalent to language-server-resolved
  code facts.
- Overwriting node attributes as a merge strategy.
- Dedup that rewrites graph identity without preserving raw observations.
- Rationale/concept nodes sharing the same namespace as code symbols.

## Recommended Canonical Vocabulary For This Repo

The following relation families are Graphify-informed but should be refined
against `rust-analyzer` and Roslyn evidence:

Structural:

- `contains`
- `declares`
- `defines`

Module/package dependency:

- `imports`
- `imports_from`
- `re_exports`
- `depends_on`
- `crate_depends_on` or a more general package-level dependency relation

Type system:

- `inherits`
- `implements`
- `extends`
- `overrides`
- `has_type`
- `parameter_type`
- `return_type`
- `field_type`
- `generic_argument`

Behavior:

- `calls`
- `constructs`
- `reads`
- `writes`
- `references`

Evidence/documentation:

- `documents`
- `rationale_for`
- `cites`

Inferred/conceptual:

- `semantically_similar_to`
- `conceptually_related_to`
- `shares_data_with`

For storage, prefer a closed enum or controlled table for canonical relations,
plus a `context` column and provider-specific metadata for details. Graphify's
framework-specific labels such as `uses_config`, `bound_to`, `listened_by`,
`uses_component`, and `navigates` should become either extension relations or
normalized combinations of relation plus context.

## First Semantic Modelling Slice Suggested By This Analysis

The most useful next implementation slice is callable/type modelling:

- Nodes: file, module/namespace, type, trait/interface, impl block, function,
  method, field/property, parameter, enum variant.
- Edges: `contains`, `defines`, `imports`, `references`, `calls`,
  `implements`, `inherits`, `parameter_type`, `return_type`, `field_type`.
- Evidence: every edge has provider, method, run, source range, confidence, and
  raw payload.
- Rust provider: `rust-analyzer`, not Graphify's tree-sitter Rust extractor.
- C# provider: `csharp-language-server`/Roslyn-backed flows, not Graphify's
  tree-sitter C# extractor.

Graphify supports this slice as a checklist of useful categories, but it also
shows why the implementation should be language-server-backed and
evidence-preserving from the start.

## Open Questions For Future ADRs

- Should `parameter_type`, `return_type`, and `field_type` be first-class
  relations, or `references` edges with structured context?
- Should rationale be stored as nodes, annotations, occurrences, or evidence?
- Should hyperedges be included in the initial SQLite schema or deferred until
  document/concept extraction exists?
- How should provider-specific relation detail be represented without allowing
  the canonical vocabulary to become arbitrary strings?
- What confidence score should name-only but syntax-observed facts receive when
  an LSP provider cannot resolve them?
- How should cross-language conceptual edges interact with strictly
  language-specific code edges in visual projections?
