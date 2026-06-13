# ADR: GPUI-Based Semantic Graph Visualization

Status: Proposed

Date: 2026-06-13

## Context

The durable graph store is expected to live in SQLite, with an optional
plain-text Data Package export for version control. The next user-facing layer
should make that graph inspectable in a fast native UI.

Graphify currently exports an interactive browser graph. That is useful, but it
is not ideal for a Rust-first local toolchain where the graph database,
incremental analysis, and code intelligence stack are all Rust-oriented.

The desired direction is a pure Rust graph viewer using GPUI, the UI framework
underlying Zed.

Official GPUI references:

- https://www.gpui.rs/
- https://github.com/zed-industries/zed/tree/main/crates/gpui

This repository also includes `gpui-flow` as a submodule at
`submodules/gpui-flow` (`cfa578758ee251657f15f281d893a08b85aedbff` at the time
of audit). It is a GPUI-native node graph editor inspired by React Flow /
xyflow, and is useful as a concrete reference for the first graph visualization
vertical slices.

The GPUI README describes it as a hybrid immediate/retained-mode,
GPU-accelerated UI framework for Rust. It also states that GPUI is still
pre-1.0 and may have breaking changes. The same README describes low-level
custom elements as the escape hatch for efficient custom views, custom layout,
and advanced rendering.

## Decision

Build the semantic graph visualizer as a native Rust GPUI application.

Use SQLite as the runtime graph source. Render a projected, visible subgraph
rather than trying to draw the entire database at once. Start with CPU layout
and GPU-accelerated rendering through GPUI. Do not attempt GPU-accelerated graph
layout in the first slices.

Use `gpui-flow` as an implementation reference, and potentially as a forked or
adapted base for the first UI slices. Do not use its in-memory `FlowState` as
the durable graph model. SQLite remains the source of truth; any `FlowNode` /
`FlowEdge` state should be a view projection.

The graph visualizer should be developed in vertical slices:

1. Simple graph visualizer.
2. Node selection with an inspector pane.
3. Edge selection with an inspector pane.

## gpui-flow Findings

`gpui-flow` is already close to the shape needed for the initial UI. Its README
lists custom node renderers, Bezier/straight/smooth-step edges, edge labels,
pan and zoom, node dragging, selection, connection handles, delete, undo/redo,
minimap, controls, viewport culling, theming, and graph utilities. See:

- `submodules/gpui-flow/README.md:12-32`

The public API exports `FlowGraph`, `FlowState`, `Minimap`, `Controls`, and all
core types from a small surface area. See:

- `submodules/gpui-flow/src/lib.rs:1-12`

The core graph types are simple and usable for a view projection:

- `NodeId` and `EdgeId` are `SharedString`.
- `FlowNode` stores ID, position, type, label, handles, selection flags,
  draggability, deletability, visibility, z-index, and measured size.
- `FlowEdge` stores ID, source, target, optional handles, edge type, selection
  flags, visibility, label, color, and stroke width.
- `Viewport` provides `flow_to_screen` and `screen_to_flow`.

See:

- `submodules/gpui-flow/src/types.rs:3-8`
- `submodules/gpui-flow/src/types.rs:92-251`
- `submodules/gpui-flow/src/types.rs:253-287`

`FlowState` is an in-memory editor store. It owns `Vec<FlowNode>` and
`Vec<FlowEdge>`, keeps a node lookup index, tracks viewport and interaction
state, and keeps undo/redo stacks by cloning whole node and edge vectors. This
is appropriate for a flow editor, but not as the canonical semantic graph
model. See:

- `submodules/gpui-flow/src/store.rs:27-75`
- `submodules/gpui-flow/src/store.rs:85-129`

The viewport and traversal helpers are directly useful. `fit_view`, `zoom_in`,
`zoom_out`, `set_center`, `get_incomers`, `get_outgoers`, and
`get_connected_edges` provide a good starting point for view behavior and
inspector queries. See:

- `submodules/gpui-flow/src/store.rs:254-361`

`FlowGraph` supports custom renderers per node type. This maps well to semantic
node kinds such as crate, module, type, trait, impl, method, function, field,
namespace, class, interface, and reference. See:

- `submodules/gpui-flow/src/graph.rs:15-38`
- `submodules/gpui-flow/src/graph.rs:59-77`

The renderer already uses an edge canvas layer, a node layer, and an edge-label
layer. It culls nodes outside the viewport before rendering them, then paints
edges on a canvas. See:

- `submodules/gpui-flow/src/graph.rs:606-744`

Edge rendering is a useful reusable piece. It supports Bezier, straight, and
smooth-step edges, per-edge color and stroke width, selected-edge styling,
arrowheads, and endpoint-based edge culling. See:

- `submodules/gpui-flow/src/edges/mod.rs:17-153`

Node selection and edge selection already exist. Node clicks update selected
state and support multi-select. Edge selection is implemented by hit-testing
edges from background mouse-down events. See:

- `submodules/gpui-flow/src/graph.rs:224-278`
- `submodules/gpui-flow/src/graph.rs:745-772`

Edge hit testing is good enough for a first slice but not a large-graph
solution. It scans every edge and approximates Bezier distance by sampling. See:

- `submodules/gpui-flow/src/edges/mod.rs:222-310`

`Minimap` and `Controls` are already separate GPUI components sharing the same
`Entity<FlowState>`. This composition maps well to a main graph viewport plus
right-side inspector pane. See:

- `submodules/gpui-flow/src/minimap.rs:9-83`
- `submodules/gpui-flow/src/controls.rs:5-77`

The stress example demonstrates 1000 nodes, viewport culling, incremental edge
addition, a minimap, controls, and a simple status overlay. It is useful as the
initial performance and composition reference, but it only caps random added
edges at 200 and does not prove large semantic-graph scale. See:

- `submodules/gpui-flow/examples/stress.rs:1-12`
- `submodules/gpui-flow/examples/stress.rs:92-190`
- `submodules/gpui-flow/examples/stress.rs:221-297`

`gpui-flow` depends on GPUI directly from the Zed git repository. Since this
repo also tracks Zed as a submodule, the graph viewer should pin or patch GPUI
to the local Zed submodule to avoid version drift. See:

- `submodules/gpui-flow/Cargo.toml:13-17`

## gpui-flow Reuse Strategy

Use these pieces directly or by adaptation:

- Viewport coordinate transforms.
- GPUI component split: graph viewport, minimap, and controls.
- Custom node renderers by semantic node kind.
- Canvas-based edge rendering.
- Bezier/straight/smooth-step edge path generation.
- Basic pan, zoom, node selection, and edge selection interactions.
- Viewport culling patterns.

Avoid carrying these editor assumptions into the semantic graph model:

- Connection handles and drag-to-connect behavior in the initial viewer.
- Delete and undo/redo behavior in the initial viewer.
- Selection flags as durable node or edge facts.
- `FlowState` as the canonical store.
- Full-vector undo snapshots for large graphs.
- Rendering all labels at all zoom levels.
- O(E) edge hit testing as the final dense-graph strategy.

Recommended adapter shape:

```text
SQLite durable graph
  -> graph query / projection layer
  -> layout table / layout engine
  -> semantic view model
  -> gpui-flow-compatible FlowNode / FlowEdge projection
  -> GPUI graph viewport
  -> inspector pane queries SQLite by selected node_id or edge_id
```

## Base Reasoning

GPUI is a reasonable fit because:

- It is native Rust.
- It is designed for Zed-level interactive UI performance.
- It supports custom low-level rendering when the normal declarative view model
  is not enough.
- The semantic graph stack is already expected to be local and Rust-centered.

The hard parts of graph visualization are not only raw drawing speed. For large
semantic graphs, the main problems are:

- layout stability,
- edge overdraw,
- label clutter,
- hit-testing cost,
- viewport culling,
- level-of-detail behavior,
- and keeping graph updates from causing disorienting layout jumps.

Because of that, the first implementation should focus on a fast viewport,
stable cached positions, and interaction quality. GPU-accelerated force layout
can be revisited later if CPU layout becomes the bottleneck.

## Architecture

```text
SQLite graph database
  -> graph query / projection layer
  -> layout cache and layout engine
  -> visible scene model
  -> GPUI graph viewport
  -> custom graph element
  -> hit testing / selection / inspector state
```

Recommended runtime model:

- Load graph nodes and edges from SQLite.
- Query only the visible or requested subgraph.
- Keep node positions in memory and optionally persist them in SQLite.
- Use a stable layout seed so nodes do not jump across runs.
- Cull offscreen nodes and edges before rendering.
- Render edges first, nodes second, labels last.
- Draw labels only at sufficient zoom or for selected/neighbor nodes.
- Keep selected node/edge state separate from graph data.

## Vertical Slice 1: Simple Graph Visualizer

Goal: render a graph from the SQLite semantic graph database.

Scope:

- Open a GPUI window.
- Load a bounded graph projection from SQLite.
- Render nodes and edges.
- Support pan and zoom.
- Use a simple initial layout.
- Keep rendering responsive for a modest graph.

Suggested constraints:

- Start with a fixed or CPU-computed layout.
- No inspector pane yet.
- No editing.
- No live LSP updates.
- No full-graph rendering requirement.

Acceptance criteria:

- The app opens quickly.
- A graph is visible.
- Pan and zoom are smooth for the initial target graph size.
- Node and edge rendering are visually distinct.
- Rendering does not require a browser or JavaScript runtime.

## Vertical Slice 2: Node Selection and Inspector

Goal: allow mouse selection of nodes and display node data.

Scope:

- Add node hit testing.
- Select a node with the mouse.
- Highlight the selected node.
- Highlight immediate neighbors.
- Display selected node details in a right-side inspector pane.

Inspector data:

- node ID,
- kind,
- name,
- qualified name,
- language,
- source file,
- source range,
- containing symbol,
- first/last seen run,
- selected node properties JSON,
- incoming/outgoing edge counts.

Acceptance criteria:

- Clicking a node selects it reliably.
- The inspector updates without reloading the graph.
- Neighbor highlighting is legible.
- Empty selection has a clear inspector state.

## Vertical Slice 3: Edge Selection and Inspector

Goal: allow mouse selection of edges and display edge data.

Scope:

- Add edge hit testing.
- Select an edge with the mouse.
- Highlight the selected edge and endpoint nodes.
- Display selected edge details in the same right-side inspector pane.

Inspector data:

- edge ID,
- source node,
- target node,
- relation,
- context,
- confidence,
- confidence score,
- weight,
- first/last seen run,
- source evidence locations,
- LSP method or provider that produced the evidence,
- raw evidence JSON when useful.

Acceptance criteria:

- Clicking near an edge selects the intended edge at normal zoom levels.
- Edge selection works when multiple edges are close together.
- The inspector distinguishes node selection from edge selection.
- Evidence rows are visible enough to explain why the edge exists.

## Rendering Strategy

Start simple:

- CPU layout.
- GPUI custom graph element.
- Cached node positions.
- Viewport culling.
- Labels only when zoomed or selected.

Avoid early over-engineering:

- Do not implement GPU force-directed layout in the first version.
- Do not build a custom shader pipeline unless GPUI's normal drawing path is
  proven insufficient.
- Do not render every label at every zoom level.
- Do not require the full database to be visible at once.

Level-of-detail rules:

- Far zoom: communities or unlabeled nodes and edges.
- Mid zoom: nodes, highlighted neighborhoods, sparse labels.
- Near zoom: labels, relation labels for selected edges, detailed hover affordances.

## Data Flow

Initial load:

1. Open SQLite database.
2. Query graph metadata and latest snapshot.
3. Load a bounded graph projection.
4. Load cached layout positions if present.
5. Compute missing positions.
6. Render the viewport.

On node selection:

1. Hit-test visible nodes.
2. Store selected node ID.
3. Query node details and adjacent edge counts.
4. Update inspector pane.
5. Repaint graph highlights.

On edge selection:

1. Hit-test visible edges.
2. Store selected edge ID.
3. Query edge details and evidence rows.
4. Update inspector pane.
5. Repaint graph highlights.

## Open Questions

- Should graph positions be stored in the main SQLite database, a separate local
  cache table, or a user-local sidecar database?
- Should the first slice render all nodes from a snapshot or require an explicit
  query/subgraph selection?
- What is the first performance target: 5k visible nodes, 20k visible nodes, or
  a smaller graph with richer labels?
- Should graph layout use community clustering from the analysis pipeline as
  the initial coarse placement?

## Risks

GPUI API stability:

GPUI is still pre-1.0, so breaking changes are expected. Keep the GPUI-specific
rendering layer isolated from graph storage and layout code.

Layout complexity:

Poor layout will make a fast renderer feel bad. Persist positions and prefer
stable incremental layout over constantly recomputing from scratch.

Hit testing:

Edge hit testing can get expensive with dense graphs. Start with viewport
culling and simple spatial indexing before considering more complex acceleration.

Label clutter:

Rendering all labels will make the graph unreadable and slow. Labels should be
level-of-detail driven.

## Consequences

Benefits:

- Native Rust visualization stack.
- No browser dependency for graph exploration.
- Fast local interaction model aligned with the rest of the planned system.
- Better path to direct SQLite integration and incremental graph updates.

Costs:

- More custom UI/rendering code than a browser-based export.
- GPUI dependency churn risk.
- Need to own layout, hit testing, and inspector interactions.

## Non-Goals

- Replacing SQLite as the durable graph source.
- Replacing Graphify-compatible JSON export.
- Implementing graph editing in the initial slices.
- Implementing GPU-accelerated graph layout in the initial slices.
- Rendering the entire database at once regardless of size.
