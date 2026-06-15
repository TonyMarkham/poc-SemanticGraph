# AGENTS.md

This file is the durable session initializer for future agents working in this
repo. Read it before making changes.

## Project Purpose

This repo is a proof of concept for a durable semantic code graph for Rust and
C# projects.

The intended pipeline is:

```text
Rust / C# workspace
  -> language-server-backed semantic extraction
  -> durable SQLite graph store
  -> optional version-control-friendly CSV snapshot
  -> local graph visualization UI
```

The project is currently design/prototype-first. The active implementation
surface includes the Rust extractor, SQLite store, checked-in `rust-analyzer`
facade, smoke-test crates, Rust visualizer JSON-RPC backend, and Blazor
WebAssembly visualizer client. `crates/wip` is the small local extraction target,
while the submodules are the main local research corpus.

## Current Decisions

Storage:

- The durable graph store is SQLite.
- The storage ADR is [adr.md](adr.md).
- Canonical graph data lives in `nodes` and `edges`.
- Source proof lives separately in `occurrences` and `edge_evidence`.
- Extraction runs track incremental refresh, soft deletion, and provenance.
- Directed edges are first-class.
- Confidence labels follow Graphify's useful convention:
  `EXTRACTED`, `INFERRED`, `AMBIGUOUS`.
- Plain-text snapshots should use Frictionless Data Package style:
  `datapackage.json` plus deterministic CSV files.
- CSV snapshots may be used to preheat SQLite; the full analysis stack then
  refreshes after code changes.

Language intelligence:

- Rust semantic facts should come from `rust-analyzer`.
- Current Rust document-symbol, reference, and call extraction use the
  checked-in `rust-analyzer` submodule crates in-process through
  `crates/rust-analyzer-lib`. Do not reintroduce runtime shelling out to
  `rust-analyzer` CLI/LSIF paths unless the user explicitly asks for that
  design change.
- C# semantic facts should come from `csharp-language-server`.
- Avoid building a bespoke semantic interrogation system when the language
  server can provide a resolved fact.
- `csharp-language-server` currently supports incoming call hierarchy, but its
  outgoing call hierarchy handler returns no result. Do not assume C# outgoing
  call edges are available from that LSP path without verifying source.

Visualization:

- The primary graph UI target is Blazor.Diagrams + Radzen.
- The visualization ADR is [adr-graph-visualization.md](adr-graph-visualization.md).
- Blazor.Diagrams owns the graph viewport.
- Radzen owns inspector/data-heavy UI: grids, forms, filters, tabs, split
  panes, and evidence tables.
- The current implemented visualization slice is browser-hosted Blazor
  WebAssembly in `apps/SemanticGraph.Visualizer`, backed by
  `crates/semantic-graph-visualizer-server`.
- The visualizer backend serves JSON-RPC 2.0 on `POST /rpc`; current methods
  are `graph.projection`, `graph.node_details`, `graph.edge_details`, and
  `graph.search_nodes`.
- Tauri is still the preferred desktop host candidate when a Rust backend is
  useful, but the current slice does not use Tauri and the Tauri/Blazor bridge
  still needs a spike.
- Do not write application-specific JavaScript for the initial UI target.
  Framework/library JS interop is acceptable where Blazor, Blazor.Diagrams,
  Radzen, or Tauri already provide it.
- GPUI/gpui-flow is a hobby/research track, not the primary product path.

## Local Research Corpus

Prefer local source inspection over web search. The important submodules are:

- `submodules/graphify`
  - Reference for graph shape, confidence, source provenance, NetworkX export,
    and graph analysis ideas.
- `submodules/rust-analyzer`
  - Source of truth for Rust LSP capabilities and semantic behavior.
- `submodules/csharp-language-server`
  - Source of truth for C# LSP capabilities and Roslyn-backed behavior.
- `submodules/Blazor.Diagrams`
  - Primary graph viewport library source.
- `submodules/tauri`
  - Candidate local desktop host/runtime source.
- `submodules/gpui-flow`
  - GPUI graph editor inspiration for the hobby/research path.
- `submodules/zed`
  - GPUI source and Zed UI architecture reference.

Use `git submodule status` to capture exact revisions when adding durable
findings to ADRs.

## Important Existing Findings

Graphify:

- Code structure extraction is tree-sitter based.
- Code-only corpora skip Graphify's LLM semantic extraction path.
- Its persisted graph format is NetworkX node-link JSON.
- Hyperedges are stored as graph metadata.
- It defaults to an undirected `nx.Graph` and preserves direction with `_src`
  and `_tgt`; this repo should model edge direction directly.
- Its merge/dedup behavior is overwrite-oriented, so durable raw evidence
  should be preserved separately from canonical nodes/edges.

Rust analyzer:

- It exposes definition, type definition, implementation, references, document
  symbols, workspace symbols, call hierarchy, and semantic tokens.
- Its outgoing call hierarchy is backed by semantic analysis.

C# language server:

- It exposes definition, references, document symbols, workspace symbols,
  implementation, type definition, semantic tokens, call hierarchy, and type
  hierarchy.
- References are backed by Roslyn `SymbolFinder.FindReferencesAsync`.
- Incoming call hierarchy is implemented.
- Outgoing call hierarchy currently returns no result.

Blazor.Diagrams:

- Supports Blazor Server and WASM.
- Separates data models from UI widgets.
- Uses Blazor/C# for most behavior and keeps JS interop small.
- Has `BlazorDiagram`, `NodeModel`, `BaseLinkModel`, selection events, SVG and
  HTML layers, routers/path generators/labels, and virtualization.
- Virtualization must be explicitly enabled and measured against semantic graph
  sizes.

Tauri:

- Treat as a candidate host for a Rust backend plus Blazor frontend.
- Verify the command bridge before committing deeply to this host shape.

## Repository Shape

Root docs:

- `adr.md`: durable SQLite storage ADR.
- `adr-graph-visualization.md`: visualization ADR.
- `AGENTS.md`: this initializer.
- `SemanticGraph.Visualizer.slnx`: solution for the Blazor visualizer client.

Rust workspace:

- `Cargo.toml`: workspace manifest.
- `crates/rust-analyzer-lib`: in-process facade over pinned `rust-analyzer`
  submodule crates.
- `crates/semantic-graph-extract`: Rust document-symbol, reference, call, and
  all-in-one extractor CLI/library.
- `crates/semantic-graph-smoke-tests`: route smoke-test/report surface.
- `crates/semantic-graph-store`: SQLite graph store and stats/demo CLI.
- `crates/semantic-graph-visualizer-server`: local read-only JSON-RPC backend
  for visualizer projection, search, and node/edge inspection.
- `crates/wip`: small Rust crate used as the local extraction target.

Applications:

- `apps/SemanticGraph.Visualizer`: Blazor WebAssembly, Radzen, and
  Blazor.Diagrams client for the read-only graph visualizer.

## Working Rules for Agents

Evidence:

- Use `rg` / `rg --files` first.
- Prefer local submodule source over web search.
- When documenting claims, include local file references where possible.
- Browse only for facts that are current, external, or not available in the
  submodule corpus.

Editing:

- Keep edits focused and reversible.
- Treat submodules as read-only research inputs unless the user explicitly asks
  to modify or update one.
- Add new external repositories under `submodules/`.
- Do not revert user changes.
- Use `apply_patch` for manual file edits.
- Preserve the repo's typed error style and `error-location` usage. Do not add
  `anyhow` to project crates.
- Prefer one Rust type per module file in project crates. Keep module files
  small and name files after the primary type they define, with `mod.rs` used to
  wire the module together.
- Do not use `super` imports or glob imports in Rust project crates. Prefer
  explicit `crate::...` and explicit item imports.

Architecture:

- Keep SQLite as the durable source of truth.
- Keep UI view models separate from durable graph records.
- Preserve raw evidence separately from canonical graph state.
- Do not add app-owned JavaScript unless the user explicitly changes that
  constraint.
- Do not make GPUI the primary UI path unless the ADR is intentionally changed.

Validation:

- For documentation-only changes, reread the changed file and inspect the diff.
- For Rust changes, run the most specific practical `cargo check` or
  `cargo test` command for the changed crate or workspace area.
- For Rust extraction route, `rust-analyzer-lib`, or smoke-test changes, also
  run the relevant smoke surface with a release binary, usually
  `cargo build --release -p semantic-graph-smoke-tests` followed by
  `./target/release/semantic-graph-smoke-tests`.
  `cargo test -p semantic-graph-smoke-tests` intentionally skips the expensive
  full-workspace references and calls smoke tests by default; run them
  explicitly with
  `SQLX_OFFLINE=true cargo test -p semantic-graph-smoke-tests -- --ignored`
  when route confidence requires it.
- For visualizer backend changes, run the most specific practical
  `cargo check -p semantic-graph-visualizer-server` or
  `cargo test -p semantic-graph-visualizer-server`.
- For visualizer client changes, run
  `dotnet build SemanticGraph.Visualizer.slnx`.
- For visualizer behavior changes, smoke the local backend/client flow where
  practical, including `graph.projection`, `graph.search_nodes`,
  `graph.node_details`, and `graph.edge_details` requests to `POST /rpc`.
- If smoke-report counts or documented route examples change, update
  `README.md` in the same change.
- Do not present Rust work as complete while `cargo check` emits warnings.
  Treat unused imports, dead code, unused public DTOs, and stale scaffolding as
  cleanup blockers unless the API is intentionally retained, documented, and
  covered by tests.
- For C# changes, run the most specific practical `dotnet build` or test.
- If validation is skipped or blocked, state exactly why.

## Useful Search Starting Points

```sh
git submodule status
rg -n "rust-analyzer-lib|rust-workspace-document-symbols|rust-workspace-references|rust-workspace-calls|rust-workspace-all|document_symbol|references|calls" crates
rg -n "semantic-graph-smoke-tests|workspace.persistence|workspace.references|workspace.calls|crate.persistence" README.md crates
rg -n "anyhow|error-location|ExtractError|GraphStoreError|RustAnalyzerLibError" crates Cargo.toml
rg -n "use super|::\\*|pub use .*::\\*" crates
rg -n "semantic-graph-visualizer-server|graph.projection|graph.node_details|graph.edge_details|graph.search_nodes|POST /rpc" README.md crates
rg -n "SemanticGraph.Visualizer|Blazor.Diagrams|Radzen|appsettings.json" README.md apps
rg -n "callHierarchy|outgoingCalls|incomingCalls" submodules/csharp-language-server
rg -n "call_hierarchy|outgoing|references|documentSymbol" submodules/rust-analyzer
rg -n "FlowGraph|FlowState|hit_test_edges|viewport|culling" submodules/gpui-flow
rg -n "BlazorDiagram|NodeModel|BaseLinkModel|SelectionChanged|Virtualization" submodules/Blazor.Diagrams
rg -n "command|invoke|ipc|webview|sql" submodules/tauri
```
