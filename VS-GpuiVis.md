# GPUI Visualizer Component Inventory

This inventory is derived from the Stitch prototypes in
`/home/tony/Downloads/stitch_graphify_node_visualizer/`:

- `graph_explorer`
- `analysis_dashboard`
- `node_inspector`
- `project_settings`
- `graphify_narrative/DESIGN.md`

The target is a GPUI desktop implementation that can reuse
`submodules/gpui-component` where practical, while keeping the graph viewport as
custom GPUI rendering.

## Foundation

| Component | Purpose | Build path | Direct use | Base component |
| --- | --- | --- | --- | --- |
| App root shell | Top-level window layer, overlays, notifications, dialogs, sheets, tooltips, and menu plumbing. | Reuse `gpui_component::Root`. | `Root` | |
| Theme tokens | Deep charcoal surfaces, electric blue, neon purple, mint green, amber, confidence colors, chart colors, sidebar/table/status tokens. | Reuse `gpui-component` theme system with a custom GraphGenie theme. | `Theme` | |
| Typography tokens | Inter for UI labels and headings, JetBrains Mono for code, metrics, IDs, and metadata. | App theme configuration plus GPUI text styles. | | `Theme` |
| Icon system | Navigation icons, graph controls, status indicators, settings affordances, inspector actions. | Reuse `gpui_component::Icon`/`IconName`; add missing SVGs if needed. | `Icon`, `IconName` | |
| Glass panel surface | Shared translucent panel style with 1px outline, tight radius, optional glow, dark fill. | App-specific wrapper around GPUI `div` styling and `gpui-component` theme tokens. | | `StyledExt`, `Theme` |
| Section header | Label-caps heading plus divider line used in inspector, dashboard, and settings sections. | Small app-specific component. | | `Label`, `Separator` |
| Status badge/chip | Confidence labels such as `EXTRACTED`, `INFERRED`, `AMBIGUOUS`, category tags, ready states. | Reuse/adapt `Badge` or `Tag`; add semantic variants. | | `Badge`, `Tag` |
| Progress indicator | Confidence score, rebuild progress, loading/indexing state. | Reuse `Progress`; custom compact metric variants. | `Progress` | |

## Global App Chrome

| Component | Purpose | Build path | Direct use | Base component |
| --- | --- | --- | --- | --- |
| Left navigation sidebar | Fixed 320px project/navigation rail with GraphGenie brand, Dashboard, Inspector, Settings, user profile. | Reuse `Sidebar`, `SidebarHeader`, `SidebarGroup`, `SidebarMenu`, `SidebarMenuItem`, `SidebarFooter`. | `Sidebar`, `SidebarHeader`, `SidebarGroup`, `SidebarMenu`, `SidebarMenuItem`, `SidebarFooter` | |
| Top toolbar | Fixed 48px header with global search, view-mode links, notifications, sync status, primary `Sync Graph` action. | App-specific composition using `Input`, `Button`, `Icon`, `Tab` or simple nav buttons. | | `Input`, `Button`, `Icon`, `Tab` |
| View mode switcher | `Cluster View`, `Isolate Path`, `Dependency Map`. | Reuse `ButtonGroup`, `TabBar`, or app-specific segmented nav. | | `ButtonGroup`, `TabBar` |
| Global search field | Search nodes by ID, label, class, or cluster. | Reuse `Input` with prefix icon and compact styling. | `Input` | |
| Footer status bar | Nodes, edges, RAM, engine latency, sync/engine status. | Reuse `StatusBar` with custom status item components. | `StatusBar` | |
| Notification/sync controls | Notification icon, cloud status, sync graph action. | Reuse `Button`, `Icon`, `Tooltip`, `Notification`. | `Button`, `Icon`, `Tooltip`, `Notification` | |

## Graph Explorer

| Component | Purpose | Build path | Direct use | Base component |
| --- | --- | --- | --- | --- |
| Graph canvas viewport | Infinite central viewport for nodes, edges, labels, grid, selection, hover, pan, zoom, and culling. | Custom GPUI `canvas` element. Use `PathBuilder`, `paint_path`, `paint_quad`, hitboxes, and mouse/wheel handlers. | | |
| Canvas grid | Subtle dark technical background grid. | Custom canvas drawing. | | |
| Graph camera controls | Zoom in, zoom out, recenter. | App-specific floating toolbar using `Button`/`Icon`/`Tooltip`. | | `Button`, `Icon`, `Tooltip` |
| Layout mode controls | Force, circular, hierarchical layout toggles. | App-specific toolbar; graph layout logic custom or library-backed. | | `ButtonGroup`, `Button`, `Icon`, `Tooltip` |
| Node glyph | Circular or diamond entity/action marker, confidence color, hover fill, selected glow, label. | Custom graph rendering component/model. | | |
| Edge glyph | Solid confirmed edge, dashed inferred edge, active path with stronger stroke and optional marching animation. | Custom graph rendering component/model. | | |
| Node label | Compact label positioned near node, hidden or reduced at low zoom. | Custom graph rendering with LOD rules. | | |
| Graph selection overlay | Selected node/edge glow, path highlighting, hover affordances. | Custom graph interaction layer. | | |
| Graph hit testing | Node click, edge click, hover, drag select, context actions. | Custom spatial index and hit-test logic. | | |
| Community legend | Floating panel with community color swatches, counts, confidence score. | App-specific panel using `List`, `Progress`, `Badge`. | | `List`, `Progress`, `Badge` |
| Mini inspector overlay | Contextual selected-node summary with UUID, degree metrics, attributes, `Trace Paths`, `Edit Props`. | App-specific panel; can reuse `Badge`, `Button`, `Progress`. | | `Badge`, `Button`, `Progress` |

## Node Inspector

| Component | Purpose | Build path | Direct use | Base component |
| --- | --- | --- | --- | --- |
| Right inspector panel | Fixed/detail panel around 500px with active node header, scrollable body, footer actions. | App-specific layout; optionally `ResizablePanel` if width should be adjustable. | | `ResizablePanel` |
| Inspector header | Active node label, title, confidence percentage, confidence/category chips, close action. | App-specific composition using `Badge`, `Button`, `Icon`. | | `Badge`, `Button`, `Icon` |
| Metadata grid | Key/value facts such as last commit, size, community, complexity. | Reuse `DescriptionList` or app-specific two-column grid. | | `DescriptionList` |
| Connectivity section | Incoming/outgoing relationship lists with counts, arrows, inferred markers. | Reuse `List` or app-specific compact list rows. | | `List` |
| Relationship row | Clickable source/target row with icon, edge confidence/type, navigation affordance. | App-specific row component. | | `ListItem`, `Icon`, `Badge` |
| Implementation preview | Syntax-highlighted source preview with copy/full-file action. | Reuse `InputState::code_editor`/`Input`; add read-only wrapper if needed. | | `Input`, `InputState::code_editor` |
| Inspector footer actions | `Refactor Edge`, share, close, trace path, edit props. | Reuse `Button`, `Icon`, `Tooltip`, `Dialog` for confirmations. | `Button`, `Icon`, `Tooltip`, `Dialog` | |

## Analysis Dashboard

| Component | Purpose | Build path | Direct use | Base component |
| --- | --- | --- | --- | --- |
| Dashboard page scaffold | Scrollable content area under shared app chrome. | App-specific layout. | | `ScrollableElement` |
| Executive summary header | Report title, label-caps section marker, stability metric. | App-specific text/header component. | | `Label`, `Badge` |
| Stat card | Total entities, active edges, inference speed, anomalies, decorative icon watermark. | App-specific card wrapper using `Icon` and theme tokens. | | `Icon`, `Theme` |
| Critical entity card | God-node card with rank, icon, title, description, connection count, related cluster swatches. | App-specific card component. | | `Icon`, `Badge`, `Tag` |
| Surprising connection list | Rows connecting two nodes with dashed/animated connector and confidence label. | App-specific list; reuse `List` if virtualized later. | | `List`, `Badge` |
| Confidence distribution chart | Extracted/inferred/ambiguous donut plus legend. | Reuse `PieChart` where possible; custom center label and legend. | | `PieChart` |
| Suggested query list | Prompt/action buttons such as bottleneck/path/failure queries. | Reuse `Button` or custom query row. | | `Button`, `Icon` |
| Node visualizer gallery | Small thumbnail/status gallery with hover overlays. | Reuse image primitives plus app-specific gallery item. | | `Badge` |

## Project Settings

| Component | Purpose | Build path | Direct use | Base component |
| --- | --- | --- | --- | --- |
| Settings page scaffold | Two-column responsive settings content under shared app chrome. | Reuse `Settings`, `SettingPage`, `SettingGroup`, `SettingItem`, `SettingField` where the layout fits; otherwise app-specific grid. | `Settings`, `SettingPage`, `SettingGroup`, `SettingItem`, `SettingField` | |
| Project status header | Project title, description, project ID chip, READY chip. | App-specific header plus `Badge`/`Tag`. | | `Badge`, `Tag` |
| Repository list | Connected repos/buckets, last synced time, add/delete actions. | Reuse `List` or app-specific rows. | | `List` |
| Repository row | Source icon, repo/bucket name, URI, sync metadata, delete button. | App-specific row component. | | `ListItem`, `Icon`, `Button` |
| Ingest option card | PDF extraction, Office files, video transcription with icon, switch, description. | App-specific card using `Switch`, `Icon`. | | `Switch`, `Icon` |
| Rebuild graph action | Full-width primary action with duration estimate and progress fill. | App-specific action component plus `Progress`. | | `Button`, `Progress` |
| Backend selector | AI provider dropdown, API key/endpoint input, visibility toggle, connection status. | Reuse `Select`, `Input`, `Button`, `Alert`/status row. | `Select`, `Input`, `Button`, `Alert` | |
| Ignore-file editor | `.graphifyignore` or similar config editor with line numbers and save/discard actions. | Reuse code editor mode, likely with custom compact styling and persistence hooks. | | `Input`, `InputState::code_editor`, `Button` |

## Popovers, Dialogs, And Utility UI

| Component | Purpose | Build path | Direct use | Base component |
| --- | --- | --- | --- | --- |
| Tooltip | Icon/button explanations for dense graph controls. | Reuse `Tooltip`. | `Tooltip` | |
| Context menu | Node, edge, graph background, and table row context actions. | Reuse `menu`/`native_menu`; app-specific action model. | `PopupMenu`, `ContextMenu`, `NativeMenu` | |
| Command/search palette | Future fast access to graph searches, saved queries, and actions. | Reuse `SearchableList`, `Dialog`, `Input`. | | `SearchableList`, `Dialog`, `Input` |
| Confirmation dialog | Delete repo, rebuild graph, refactor edge, destructive settings changes. | Reuse `Dialog`/`AlertDialog`. | `Dialog`, `AlertDialog` | |
| Sheet/popup inspector | Optional alternate inspector presentation for narrow windows. | Reuse `Sheet` or `Popover`. | `Sheet`, `Popover` | |
| Loading/skeleton state | Graph loading, dashboard loading, settings save/rebuild progress. | Reuse `Spinner`, `Skeleton`, `Progress`, `Notification`. | `Spinner`, `Skeleton`, `Progress`, `Notification` | |

## Graph-Specific Models And Services

| Component | Purpose | Build path | Direct use | Base component |
| --- | --- | --- | --- | --- |
| Graph view model | Projection-specific nodes, edges, positions, colors, labels, confidence, community IDs. | App-specific Rust model separate from durable DB records. | | |
| Layout engine adapter | Force/circular/hierarchical layout selection and parameterization. | Custom adapter; can wrap an external layout algorithm later. | | |
| Viewport controller | Pan, zoom, recenter, fit selection, coordinate transforms. | Custom GPUI controller. | | |
| Selection controller | Selected node/edge/path, multi-select, hover state, keyboard navigation. | Custom app state. | | |
| Graph renderer | Draw order, culling, LOD, labels, glow, active paths, dashed edges. | Custom GPUI canvas renderer. | | |
| Graph interaction layer | Hit testing, drag, click, context menu, hover tooltips, keyboard shortcuts. | Custom with GPUI events and spatial indexing. | | |
| Query/action model | Search, isolate path, dependency map, trace paths, refactor edge. | App-specific commands connected to visualizer backend. | | |

## Reuse Summary

High-confidence reuse from `gpui-component`:

- `Root`, `Theme`, `Icon`, `Button`, `ButtonGroup`
- `Sidebar`, `StatusBar`, `Tooltip`, `Popover`, `Dialog`, `Sheet`
- `Input`, code editor mode, `Select`, `Switch`, `Checkbox`
- `Badge`, `Tag`, `Progress`, `Spinner`, `Skeleton`, `Alert`
- `List`, `SearchableList`, `DataTable`, `VirtualList`
- `Chart` components, especially `PieChart`, `LineChart`, `BarChart`, `AreaChart`
- `Settings` and `Form` components where their structure matches the settings screen

Custom GPUI work required:

- Main graph viewport and all node-link rendering
- Graph layout, culling, hit testing, pan/zoom, selection, and active path animation
- Stitch-specific glass panel wrappers and dense pro-tool component variants
- App-specific graph query actions and backend integration

Open validation item:

- Direct crate reuse needs a GPUI version-alignment spike because
  `gpui-component` currently pins a different Zed/GPUI commit than this repo's
  `submodules/zed`.
