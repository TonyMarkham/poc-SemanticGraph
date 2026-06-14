# Semantic Analyzer Missing Features

This project already has useful semantic graph capabilities: Rust document
symbols, Rust references, Rust outgoing calls, durable SQLite storage, evidence
rows, stale-route handling, and a read-only visualizer surface.

The missing work below is about turning that foundation into a stronger
semantic analyzer. It is not a plan to duplicate full-text search. `rg` remains
the better tool for raw text search.

## Query Capabilities

- First-class semantic query CLI.
  - Current state: useful answers can be obtained with hand-written SQL.
  - Missing: named queries for common audits such as non-pattern result types,
    production-only API checks, dependency fan-in/fan-out, stale facts, and
    unresolved semantic evidence.

- Stable query output formats.
  - Current state: store stats and visualizer DTOs exist.
  - Missing: deterministic tabular/JSON output for semantic audits so results
    can be used in CI or compared across runs.

- Production/test/source-set classification.
  - Current state: file paths can be filtered manually.
  - Missing: normalized classification stored on files or symbols, such as
    `production`, `test`, `fixture`, `generated`, and `example`.

- Crate/package ownership model.
  - Current state: workspace files are stored, but packages/crates are not
    first-class graph entities.
  - Missing: durable crate/package rows and edges from files/symbols to their
    package owner.

## Rust Semantic Coverage

- Definition/type-definition/implementation edges.
  - Current state: document symbols, references, and calls are persisted.
  - Missing: directed edges for `definition`, `type_definition`,
    `implementation`, trait implementation, override, and related navigation
    facts.

- Type-use extraction.
  - Current state: symbol signatures may contain type text in document-symbol
    detail, but type usages are not first-class facts.
  - Missing: resolved type references in signatures, fields, local bindings,
    generic parameters, return types, trait bounds, and where clauses.

- Import/use alias resolution.
  - Current state: raw text can show `Result`, and document-symbol detail can
    show some fully qualified paths.
  - Missing: resolved `use` trees and alias edges, so queries can prove whether
    `Result` means `std::result::Result`, a crate alias, a shadowed local type,
    or a re-export.

- Public API surface classification.
  - Current state: document symbols include names and ranges.
  - Missing: visibility, exported/public API status, module privacy, and
    re-export information as queryable fields.

- Macro-aware semantic facts.
  - Current state: facts come from rust-analyzer, but macro-origin metadata is
    not modeled deeply.
  - Missing: whether a symbol/use/call came from source text, macro expansion,
    derive output, or generated code, with evidence.

- Stronger symbol identity.
  - Current state: durable symbol keys are derived from workspace-relative
    paths and symbol hierarchy.
  - Missing: richer rust-analyzer-backed identity where available, especially
    for overloaded names, impl items, trait items, re-exports, and generated
    symbols.

## Error And API Pattern Audits

- Typed result alias audit.
  - Current state: the graph can find many function signatures containing
    `std::result::Result` from document-symbol detail and file context.
  - Missing: resolved type-path audit that understands aliases and imports and
    can distinguish `Result<T, CrateError>` from crate-specific aliases such as
    `ExtractResult<T>`.

- Error variant coverage.
  - Current state: error enums and constructors are visible as symbols.
  - Missing: queries that map each `?`/error conversion site to a specific
    error variant or conversion path.

- Cross-crate error boundary checks.
  - Current state: crates have local typed error conventions.
  - Missing: semantic checks that enforce those conventions at crate public
    boundaries and executable boundaries.

## Evidence And Provenance

- Evidence completeness reporting.
  - Current state: occurrences and edge evidence are stored.
  - Missing: route-level reports that identify symbols/edges without enough
    evidence, skipped semantic targets, and facts that were inferred rather
    than directly extracted.

- Source snippets as optional evidence.
  - Current state: ranges and raw provider JSON are stored.
  - Missing: optional compact snippets around evidence ranges for review
    surfaces. This should stay bounded and should not become full source
    indexing.

- Query-time evidence joins.
  - Current state: SQL can join nodes, edges, occurrences, and evidence.
  - Missing: reusable views or APIs that return "fact plus proof" records for
    audits and UI consumption.

## Incremental And Freshness Behavior

- Route dependency model.
  - Current state: document symbols, references, and calls have route status and
    stale closing.
  - Missing: explicit route dependency metadata, so relation routes can declare
    they require a current document-symbol graph.

- Incremental file refresh.
  - Current state: runs can close stale rows, but extraction is still mostly
    batch-oriented.
  - Missing: targeted refresh for changed files and affected symbols.

- Cross-route stale policy.
  - Current state: stale nodes and edges can be closed per route.
  - Missing: policy for what happens when a document-symbol change invalidates
    reference/call/type facts owned by other routes.

## C# Semantic Coverage

- C# extractor implementation.
  - Current state: C# language server research exists, but extraction is not
    implemented.
  - Missing: C# document symbols, references, definitions, incoming calls, and
    any supported type hierarchy facts.

- C# outgoing calls verification.
  - Current state: local findings indicate outgoing call hierarchy may not be
    implemented by the selected C# language-server path.
  - Missing: verified alternative extraction route or documented limitation.

- Cross-language graph model.
  - Current state: schema allows multiple languages.
  - Missing: language-specific normalizers and shared query semantics across
    Rust and C#.

## Visualization And Review UI

- Audit views.
  - Current state: visualizer supports projection, search, node details, and
    edge details.
  - Missing: dedicated views for semantic audits, including result-type
    violations, unresolved references, high fan-in symbols, stale facts, and
    route coverage.

- Evidence-first inspection.
  - Current state: edge evidence is visible.
  - Missing: audit result pages that put source proof, route, confidence, and
    freshness beside each finding.

- Graph scaling controls.
  - Current state: projection is bounded.
  - Missing: better slicing by crate, route, relation, symbol kind, confidence,
    and production/test classification.

## Snapshots And CI

- CSV/data-package snapshots.
  - Current state: SQLite is the durable source of truth.
  - Missing: deterministic Frictionless Data Package snapshots for review,
    versioning, and DB preheating.

- CI-friendly audit commands.
  - Current state: smoke routes and stats exist.
  - Missing: commands that exit nonzero on configured semantic policy
    violations.

- Baseline/diff workflow.
  - Current state: runs can be compared manually by stats and SQL.
  - Missing: first-class comparison between two extraction runs or two DBs.

## Non-Goals

- Full-text source indexing.
  - Raw source text search is better handled by tools like `rg`.
  - The semantic graph should store facts, evidence, and bounded proof, not
    serialize the whole codebase into SQLite.

- Reimplementing language-server semantics.
  - When rust-analyzer or another language server can provide a resolved fact,
    extraction should use that fact rather than building a parallel semantic
    engine.
