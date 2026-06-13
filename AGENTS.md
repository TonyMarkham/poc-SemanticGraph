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

The project is currently design/prototype-first. The implementation surface is
small (`crates/wip`), while the submodules are the main local research corpus.

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
- Tauri is the preferred host candidate when a Rust backend is useful, but the
  Tauri/Blazor bridge still needs a spike.
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

Rust workspace:

- `Cargo.toml`: workspace manifest.
- `crates/wip`: current placeholder Rust crate.

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
- For C# changes, run the most specific practical `dotnet build` or test.
- If validation is skipped or blocked, state exactly why.

## Useful Search Starting Points

```sh
git submodule status
rg -n "callHierarchy|outgoingCalls|incomingCalls" submodules/csharp-language-server
rg -n "call_hierarchy|outgoing|references|documentSymbol" submodules/rust-analyzer
rg -n "FlowGraph|FlowState|hit_test_edges|viewport|culling" submodules/gpui-flow
rg -n "BlazorDiagram|NodeModel|BaseLinkModel|SelectionChanged|Virtualization" submodules/Blazor.Diagrams
rg -n "command|invoke|ipc|webview|sql" submodules/tauri
```
