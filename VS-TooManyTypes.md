# Rust Files With Too Many Types

Source: SemanticGraph MCP server at
`.refactor-radar/bin/semantic-graph-mcp-server`.

MCP tools used: `graph_stats`, `graph_file_summary`.

Counted MCP symbol kinds: `struct`, `enum`, `interface`.

Excluded MCP symbol kind: `object`, because Rust impl blocks are represented as
`object` in this graph.

Checked: 400 Rust files present in the MCP graph.

## Violations

- `crates/semantic-graph-extract/src/providers/csharp_ls/csharp_ls_provider.rs`
  - 8 types:
  - `struct CSharpLsProvider`
  - `struct CallGroup`
  - `struct CallableTargetContext`
  - `struct FileSymbolContext`
  - `struct MappedCallTarget`
  - `struct ReferenceGroup`
  - `struct ReferenceTargetContext`
  - `struct SymbolIndex`
- `crates/semantic-graph-extract/src/providers/rust_analyzer/rust_analyzer_provider.rs`
  - 8 types:
  - `struct CallGroup`
  - `struct CallableTargetContext`
  - `struct FileSymbolContext`
  - `struct MappedCallTarget`
  - `struct ReferenceGroup`
  - `struct ReferenceTargetContext`
  - `struct RustAnalyzerProvider`
  - `struct SymbolIndex`
- `crates/semantic-graph-mcp-server/src/tools/neighbors.rs`
  - 2 types:
  - `enum NeighborDirectionParam`
  - `struct NeighborsParams`
- `crates/semantic-graph-mcp-server/tests/stdio.rs`
  - 2 types:
  - `struct Fixture`
  - `struct StdioServer`
- `crates/semantic-graph-query/tests/graph_query_service.rs`
  - 6 types:
  - `interface HasNodeId`
  - `interface NodeDetailsContainer`
  - `struct EdgeFixtureInput`
  - `struct Fixture`
  - `struct FixtureIds`
  - `struct OccurrenceFixtureInput`
- `crates/semantic-graph-store/src/main.rs`
  - 2 types:
  - `enum Command`
  - `struct Cli`
- `crates/wip/src/models.rs`
  - 4 types:
  - `enum WidgetState`
  - `struct AuditNote`
  - `struct Widget`
  - `struct WidgetId`
- `crates/wip/src/pipeline.rs`
  - 4 types:
  - `interface RenderSummary`
  - `interface WidgetStore`
  - `struct MemoryWidgetStore`
  - `struct WidgetProcessor`

## Not In Graph

These Rust candidate paths existed in the working tree but were not present in
the MCP graph:

- `__SmokeTestAssets__/wip-a/lib.rs`
- `__SmokeTestAssets__/wip-b/foo_bar.rs`
- `__SmokeTestAssets__/wip-b/foo_bar_baz.rs`
- `__SmokeTestAssets__/wip-b/lib.rs`
- `__SmokeTestAssets__/wip-c/foo_bar.rs`
- `__SmokeTestAssets__/wip-c/foo_bar_baz.rs`
