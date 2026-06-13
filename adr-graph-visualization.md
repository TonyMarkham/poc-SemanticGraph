# ADR: Blazor.Diagrams-Based Semantic Graph Visualization

Status: Proposed

Date: 2026-06-13

## Context

The durable graph store is expected to live in SQLite, with an optional
plain-text Data Package export for version control. The next user-facing layer
should make that graph inspectable in a fast local UI.

Graphify currently exports an interactive browser graph. That is useful as
inspiration, but the project needs a durable, local semantic graph viewer that
can inspect Rust and C# code graphs produced from language-server evidence.

The primary UI target is now a Blazor-based graph application:

- Blazor.Diagrams for the semantic graph viewport.
- Radzen Blazor Components for inspector panes, grids, forms, filters, tabs,
  split panes, and operational UI around the graph.
- A local host/backend for SQLite, LSP orchestration, graph projection, and
  layout work. Tauri remains the preferred host candidate when a Rust backend
  is useful.

This repository includes `Blazor.Diagrams` as a submodule at
`submodules/Blazor.Diagrams` (`fceb36fed10b3fcb1a4601dd199871326075d2ae`,
tag `3.0.4.1` at the time of audit). It also includes `gpui-flow` and `zed` as
submodules for a GPUI-based alternative, but that path is no longer the primary
product target.

Relevant references:

- `submodules/Blazor.Diagrams/README.md:5`
- `submodules/Blazor.Diagrams/README.md:20-44`
- `submodules/gpui-flow/README.md:12-32`
- https://blazor.radzen.com/

## Decision

Build the primary semantic graph visualizer as a Blazor application using
Blazor.Diagrams for graph rendering and Radzen for the surrounding inspector
and data-heavy UI.

SQLite remains the source of truth. Blazor.Diagrams models are view models, not
durable graph records. The app should load bounded graph projections from the
SQLite graph database, map them into diagram node/link models, and query SQLite
again for inspector details when a node or edge is selected.

Do not write application-specific JavaScript for the initial implementation.
Framework and library JavaScript interop is acceptable where it is already part
of Blazor, Blazor.Diagrams, Radzen, or the selected desktop host. If Tauri is
used, keep any host bridge thin and explicit.

Keep the GPUI/gpui-flow direction as a separate hobby or research track. It may
still be valuable for a pure Rust, Zed-like renderer, but it should not block or
compete with the primary Blazor.Diagrams implementation.

The graph visualizer should be developed in vertical slices:

1. Simple graph visualizer.
2. Node selection with an inspector pane.
3. Edge selection with an inspector pane.

## Primary Stack

Recommended primary stack:

```text
Local desktop host
  -> Rust or .NET backend commands
      -> SQLite graph store
      -> LSP orchestration
      -> graph projection queries
      -> layout cache / layout computation
  -> Blazor WASM frontend
      -> Blazor.Diagrams graph viewport
      -> Radzen inspector, grids, filters, forms, and panels
```

Tauri is still a good host candidate because it keeps the storage, LSP, and
projection backend in Rust. The first implementation should verify the
Tauri-to-Blazor command bridge early. If that bridge requires too much
application-owned JavaScript, consider keeping the backend boundary in .NET or
using another Blazor-friendly local host. The product decision is
Blazor.Diagrams + Radzen for the UI, not JavaScript-heavy web development.

## Blazor.Diagrams Findings

Blazor.Diagrams is directly aligned with the needed graph viewport. Its README
describes it as a customizable and extensible diagramming library for Blazor
Server and Blazor WASM. See:

- `submodules/Blazor.Diagrams/README.md:5`

Its stated goals match this project well:

- performance matters, especially in WebAssembly;
- the data/model layer is separated from the UI/widget layer;
- UI can be customized with Blazor components and CSS;
- JavaScript is intentionally minimized and used only when necessary.

See:

- `submodules/Blazor.Diagrams/README.md:22-26`

The feature list covers most of the initial semantic graph UI needs:

- SVG layer for links and nodes;
- HTML layer for nodes;
- links between nodes, ports, and links;
- link routers, path generators, markers, and labels;
- pan, zoom, and zoom-to-fit;
- multi-selection and region selection;
- custom nodes, links, and groups;
- customizable diagram overview/navigator;
- virtualization;
- read-only locking;
- algorithms package.

See:

- `submodules/Blazor.Diagrams/README.md:30-44`

The local source exposes a `BlazorDiagram` type built on the core `Diagram`
model and supports model-to-component registration for custom rendering. See:

- `submodules/Blazor.Diagrams/src/Blazor.Diagrams/BlazorDiagram.cs:9-26`
- `submodules/Blazor.Diagrams/src/Blazor.Diagrams/BlazorDiagram.cs:28-66`

The core `Diagram` has first-class layers for nodes, links, groups, controls,
pan, zoom, selection events, pointer events, and changed events. Default
behaviors include selection, dragging, new-link dragging, panning, zooming,
keyboard shortcuts, controls, and virtualization. See:

- `submodules/Blazor.Diagrams/src/Blazor.Diagrams.Core/Diagram.cs:22-37`
- `submodules/Blazor.Diagrams/src/Blazor.Diagrams.Core/Diagram.cs:43-80`
- `submodules/Blazor.Diagrams/src/Blazor.Diagrams.Core/Diagram.cs:56-68`

Selection is already modeled for nodes and links through selectable models and
`SelectionChanged`. That directly supports the node and edge inspector slices.
See:

- `submodules/Blazor.Diagrams/src/Blazor.Diagrams.Core/Diagram.cs:106-169`

The canvas renders a dedicated SVG layer for links and SVG nodes, and a
separate HTML layer for normal nodes and groups. This is useful because graph
edges can stay in SVG while semantic node cards can be ordinary Blazor
components. See:

- `submodules/Blazor.Diagrams/src/Blazor.Diagrams/Components/DiagramCanvas.razor:1-58`

Node and link models already contain the concepts needed for a semantic graph
projection. `NodeModel` has identity, position, size, ports, links, movement,
and bounds. `BaseLinkModel` has source/target anchors, route/path generation,
markers, vertices, labels, and bounds. See:

- `submodules/Blazor.Diagrams/src/Blazor.Diagrams.Core/Models/NodeModel.cs:9-45`
- `submodules/Blazor.Diagrams/src/Blazor.Diagrams.Core/Models/NodeModel.cs:94-120`
- `submodules/Blazor.Diagrams/src/Blazor.Diagrams.Core/Models/Base/BaseLinkModel.cs:10-43`
- `submodules/Blazor.Diagrams/src/Blazor.Diagrams.Core/Models/Base/BaseLinkModel.cs:59-70`
- `submodules/Blazor.Diagrams/src/Blazor.Diagrams.Core/Models/Base/BaseLinkModel.cs:122-141`

Virtualization is built in but must be explicitly enabled and validated against
semantic graph sizes. The behavior listens to zoom, pan, and container changes
and toggles model visibility based on bounds. See:

- `submodules/Blazor.Diagrams/src/Blazor.Diagrams/Options/BlazorDiagramOptions.cs:10-14`
- `submodules/Blazor.Diagrams/src/Blazor.Diagrams.Core/Options/DiagramVirtualizationOptions.cs:3-8`
- `submodules/Blazor.Diagrams/src/Blazor.Diagrams.Core/Behaviors/VirtualizationBehavior.cs:5-69`

Blazor.Diagrams does contain a small JavaScript interop surface for DOM bounds
and resize observation. That is acceptable for the primary path because it is
library-owned infrastructure, not application-specific graph logic. See:

- `submodules/Blazor.Diagrams/src/Blazor.Diagrams/Extensions/JSRuntimeExtensions.cs:9-32`

## Radzen Role

Radzen should be used for the operational UI around the graph, not for the
graph viewport itself.

Recommended Radzen-owned surfaces:

- right-edge node/edge inspector;
- evidence tables;
- source occurrence tables;
- graph filters;
- workspace/snapshot selectors;
- tabs for overview, properties, evidence, and raw JSON;
- split panes or panels around the diagram;
- forms for saved graph queries or projection settings.

The graph surface itself should remain Blazor.Diagrams so node/link behavior,
selection, routing, labels, overview, and virtualization stay in one diagram
model.

## GPUI/gpui-flow Hobby Track

GPUI and `gpui-flow` remain interesting for a pure Rust renderer, especially
for learning from Zed's native UI direction. They should be treated as a hobby
or research track rather than the primary implementation plan.

`gpui-flow` is a GPUI-native node graph editor with custom node renderers,
Bezier/straight/smooth-step edges, labels, pan/zoom, selection, minimap,
controls, viewport culling, theming, and graph utilities. See:

- `submodules/gpui-flow/README.md:12-32`

The useful ideas to keep from `gpui-flow` are:

- viewport coordinate transforms;
- graph viewport, minimap, and controls as separate components;
- custom node rendering by semantic kind;
- canvas-based edge rendering;
- basic pan, zoom, node selection, and edge selection interactions;
- viewport culling patterns.

Do not use `gpui-flow` as the main product path for now. It would require more
custom UI work for the inspector, tables, filters, and operational surfaces
that Radzen and Blazor already provide.

## Base Reasoning

Blazor.Diagrams + Radzen is the most pragmatic primary path because it starts
from two pieces close to the target experience:

- Blazor.Diagrams already solves most graph viewport mechanics.
- Radzen already solves much of the inspector and data-heavy application UI.
- The application can be written primarily in C# and Razor on the frontend.
- SQLite, LSP orchestration, and graph projection can remain behind a local
  backend boundary.
- The initial implementation can avoid application-specific JavaScript.

The GPUI path is technically attractive, especially for a pure Rust, Zed-like
native graph renderer. The cost is that it turns many ordinary product UI
surfaces into custom work. That makes it better as a parallel experiment than
as the first product target.

The hard parts of semantic graph visualization are still present regardless of
UI stack:

- layout stability;
- edge overdraw;
- label clutter;
- hit-testing cost;
- viewport culling;
- level-of-detail behavior;
- keeping graph updates from causing disorienting layout jumps.

Because of that, the first implementation should focus on bounded graph
projections, stable cached positions, useful selection/inspection workflows,
and good level-of-detail rules. Do not try to render the entire database at
once.

## Architecture

```text
SQLite graph database
  -> graph query / projection layer
  -> layout cache and layout engine
  -> semantic graph view model
  -> Blazor.Diagrams NodeModel / BaseLinkModel projection
  -> Blazor.Diagrams viewport
  -> Radzen inspector and data panels
```

Recommended runtime model:

- Load graph nodes and edges from SQLite through a backend service or host
  command.
- Query only the visible, selected, or requested subgraph.
- Keep node positions in memory and optionally persist them in SQLite.
- Use a stable layout seed so nodes do not jump across runs.
- Map semantic nodes to custom Blazor.Diagrams node models/components.
- Map semantic edges to link models with labels, markers, and relation styling.
- Enable and validate Blazor.Diagrams virtualization.
- Draw labels only at sufficient zoom or for selected/neighbor nodes.
- Keep selected node/edge IDs separate from durable graph data.
- Use Radzen to display inspector details queried from SQLite.

## Vertical Slice 1: Simple Graph Visualizer

Goal: render a graph from the SQLite semantic graph database.

Scope:

- Open the local Blazor graph application.
- Load a bounded graph projection from SQLite.
- Render nodes and edges in Blazor.Diagrams.
- Support pan and zoom.
- Use a simple initial layout.
- Keep rendering responsive for a modest graph.

Suggested constraints:

- Start with a fixed or CPU-computed layout.
- No inspector pane yet.
- No editing.
- No live LSP updates.
- No full-graph rendering requirement.
- No application-specific JavaScript.

Acceptance criteria:

- The app opens quickly.
- A graph is visible.
- Pan and zoom are smooth for the initial target graph size.
- Node and edge rendering are visually distinct.
- The graph is rendered through Blazor.Diagrams.

## Vertical Slice 2: Node Selection and Inspector

Goal: allow mouse selection of nodes and display node data.

Scope:

- Use Blazor.Diagrams selection events for node selection.
- Select a node with the mouse.
- Highlight the selected node.
- Highlight immediate neighbors.
- Display selected node details in a right-side Radzen inspector pane.

Inspector data:

- node ID;
- kind;
- name;
- qualified name;
- language;
- source file;
- source range;
- containing symbol;
- first/last seen run;
- selected node properties JSON;
- incoming/outgoing edge counts.

Acceptance criteria:

- Clicking a node selects it reliably.
- The inspector updates without reloading the graph.
- Neighbor highlighting is legible.
- Empty selection has a clear inspector state.

## Vertical Slice 3: Edge Selection and Inspector

Goal: allow mouse selection of edges and display edge data.

Scope:

- Use Blazor.Diagrams link selection for edge selection.
- Select an edge with the mouse.
- Highlight the selected edge and endpoint nodes.
- Display selected edge details in the same right-side Radzen inspector pane.

Inspector data:

- edge ID;
- source node;
- target node;
- relation;
- context;
- confidence;
- confidence score;
- weight;
- first/last seen run;
- source evidence locations;
- LSP method or provider that produced the evidence;
- raw evidence JSON when useful.

Acceptance criteria:

- Clicking near an edge selects the intended edge at normal zoom levels.
- Edge selection works when multiple edges are close together.
- The inspector distinguishes node selection from edge selection.
- Evidence rows are visible enough to explain why the edge exists.

## Rendering Strategy

Start simple:

- CPU layout.
- Blazor.Diagrams node/link projection.
- Cached node positions.
- Blazor.Diagrams virtualization enabled and measured.
- Labels only when zoomed or selected.
- Radzen inspector and tables outside the graph viewport.

Avoid early over-engineering:

- Do not implement GPU force-directed layout in the first version.
- Do not build a custom JavaScript graph renderer.
- Do not render every label at every zoom level.
- Do not require the full database to be visible at once.
- Do not implement graph editing in the initial slices.

Level-of-detail rules:

- Far zoom: communities or unlabeled nodes and edges.
- Mid zoom: nodes, highlighted neighborhoods, sparse labels.
- Near zoom: labels, relation labels for selected edges, detailed hover
  affordances.

## Data Flow

Initial load:

1. Open SQLite database.
2. Query graph metadata and latest snapshot.
3. Load a bounded graph projection.
4. Load cached layout positions if present.
5. Compute missing positions.
6. Build Blazor.Diagrams node/link models.
7. Render the viewport.

On node selection:

1. Receive Blazor.Diagrams selection event.
2. Store selected node ID in view state.
3. Query node details and adjacent edge counts.
4. Update Radzen inspector pane.
5. Repaint graph highlights.

On edge selection:

1. Receive Blazor.Diagrams link selection event.
2. Store selected edge ID in view state.
3. Query edge details and evidence rows.
4. Update Radzen inspector pane.
5. Repaint graph highlights.

## Open Questions

- Should graph positions be stored in the main SQLite database, a separate local
  cache table, or a user-local sidecar database?
- Should the first slice render all nodes from a snapshot or require an
  explicit query/subgraph selection?
- What is the first performance target: 1k visible nodes, 5k visible nodes, or
  a smaller graph with richer labels?
- Should graph layout use community clustering from the analysis pipeline as
  the initial coarse placement?
- If Tauri is used, what is the cleanest C# JS interop wrapper for Tauri
  commands that avoids application-specific JavaScript modules?
- Should the backend be Rust-first through Tauri, .NET-first for easier Blazor
  integration, or split by responsibility?

## Risks

SVG/HTML scale ceiling:

Blazor.Diagrams uses SVG and HTML layers. This is excellent for productivity
and custom components, but it may hit a ceiling earlier than a GPUI canvas or
WebGL renderer. Use bounded projections, virtualization, and level-of-detail
rules before increasing graph size.

Host bridge complexity:

Tauri plus Blazor WASM is plausible, but the command bridge must be proven
early. If it requires too much custom JavaScript, use a thinner interop wrapper
or reconsider the host boundary.

Library-owned JavaScript interop:

Blazor.Diagrams intentionally minimizes JavaScript, but it still uses JS interop
for bounds and resize observation. This is acceptable, but it means the primary
path is "no application-specific JavaScript", not "no JavaScript exists in the
runtime".

Layout complexity:

Poor layout will make a useful renderer feel bad. Persist positions and prefer
stable incremental layout over constantly recomputing from scratch.

Label clutter:

Rendering all labels will make the graph unreadable and slow. Labels should be
level-of-detail driven.

## Consequences

Benefits:

- Primary graph UI starts from a purpose-built Blazor diagramming library.
- Inspector, evidence tables, filters, forms, and panels can use Radzen instead
  of custom native UI.
- Frontend implementation can stay mostly in C# and Razor.
- No application-specific JavaScript is required for the initial target.
- SQLite remains the durable source of truth.
- GPUI remains available for separate Rust-native experimentation.

Costs:

- The primary UI uses a WebView/WASM/browser-style runtime instead of a pure
  native GPUI renderer.
- Blazor.Diagrams SVG/HTML rendering may cap practical graph size.
- Tauri/Blazor host integration must be validated.
- The app depends on Blazor.Diagrams, Radzen, and the chosen host framework.

## Non-Goals

- Replacing SQLite as the durable graph source.
- Replacing Graphify-compatible JSON export.
- Implementing graph editing in the initial slices.
- Implementing GPU-accelerated graph layout in the initial slices.
- Writing a custom JavaScript graph renderer.
- Rendering the entire database at once regardless of size.
- Making GPUI/gpui-flow the primary implementation path.
