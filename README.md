# SemanticGraph Prototype

SemanticGraph is a proof of concept for extracting code facts into a durable
SQLite graph.

Right now, the useful path is Rust document-symbol extraction and workspace
reference extraction through the checked-in `rust-analyzer` libraries. The
extractor supports single-file, crate-scoped, workspace-scoped, and
workspace-reference routes. A read-only visualizer slice with projection,
search, selection inspection, and evidence display is available through a Rust
JSON-RPC backend and a Blazor WebAssembly client.

## Extract One Rust File

Use this when you want to add or refresh facts for a file in a database. If the
database already exists, the extractor reuses it and records another extraction
run.

From the repo root:

```sh
cargo run -p semantic-graph-extract -- rust-document-symbols \
  --db .local/rust-extract-wip.db \
  --workspace-root . \
  --package-path crates/wip \
  --file crates/wip/src/lib.rs
```

This creates or updates `.local/rust-extract-wip.db`.

You do not need to initialize the database first. The extractor opens the
SQLite database, runs migrations, loads the pinned `rust-analyzer` libraries
in-process through `rust-analyzer-lib`, extracts symbols for the requested
file, and writes the graph rows.

Example successful output:

```text
workspace=1 run=1 files=1 nodes=4 edges=3 occurrences=3 evidence=3
```

## Inspect The Result

Print row counts from the SQLite database:

```sh
cargo run -p semantic-graph-store -- stats \
  --db .local/rust-extract-wip.db
```

Expected shape after extracting `crates/wip/src/lib.rs`:

```text
workspaces=1
extraction_runs=1
files=1
nodes=4
edges=3
occurrences=3
edge_evidence=3
```

## Extract A Different File

Use the same command and change `--file`:

```sh
cargo run -p semantic-graph-extract -- rust-document-symbols \
  --db .local/rust-extract-wip.db \
  --workspace-root . \
  --package-path crates/wip \
  --file crates/wip/src/models.rs
```

Each run records a new extraction run. Canonical file, node, and edge rows are
upserted, while occurrence and evidence rows are inserted as run proof.

## Extract A Rust Crate

Use this when you want to extract every Rust source file that `rust-analyzer`
indexes for a package path:

```sh
cargo run -p semantic-graph-extract -- rust-crate-document-symbols \
  --db .local/rust-crate-extract-wip.db \
  --workspace-root . \
  --package-path crates/wip
```

The crate route loads the workspace through `rust-analyzer-lib`, discovers Rust
source files for the package path, then extracts hierarchical document-symbol
data for each discovered file in one extraction run.

Example successful output for `crates/wip`:

```text
workspace=1 run=1 files=4 nodes=57 edges=53 occurrences=53 evidence=53
```

Inspect the result:

```sh
cargo run -p semantic-graph-store -- stats \
  --db .local/rust-crate-extract-wip.db
```

Expected shape:

```text
workspaces=1
extraction_runs=1
files=4
nodes=57
edges=53
occurrences=53
edge_evidence=53
```

## Extract A Rust Workspace

Use this when you want to extract every Rust source file that `rust-analyzer`
indexes for the workspace rooted at `--workspace-root`:

```sh
cargo run -p semantic-graph-extract -- rust-workspace-document-symbols \
  --db .local/rust-workspace-extract.db \
  --workspace-root .
```

The workspace route uses the same `rust-analyzer-lib` source discovery path, but
treats the workspace root as the extraction boundary. In this repo, that
excludes `submodules/` because those crates are not part of the root Cargo
workspace.

Example successful output for the current repo workspace:

```text
workspace=1 run=1 files=147 nodes=1337 edges=1190 occurrences=1190 evidence=1190
```

Inspect the result:

```sh
cargo run -p semantic-graph-store -- stats \
  --db .local/rust-workspace-extract.db
```

Expected shape:

```text
workspaces=1
extraction_runs=1
files=147
nodes=1337
edges=1190
occurrences=1190
edge_evidence=1190
```

## Extract Rust Workspace References

Use this when you want current Rust symbols and current Rust `references`
edges in one run:

```sh
cargo run -p semantic-graph-extract -- rust-workspace-references \
  --db .local/rust-workspace-references.db \
  --workspace-root .
```

The references route refreshes document symbols first, then queries
`rust-analyzer` references for eligible workspace symbols. Reference edges are
stored as directed `source --references--> target` edges with occurrence and
edge evidence proof.

Example successful output for the current repo workspace:

```text
workspace=1 run=1 files=147 nodes=1337 contains_edges=1190 references_edges=2484 reference_occurrences=2968 evidence=4158 routes_complete=148 stale_nodes_closed=0 stale_edges_closed=0
```

Expected shape:

```text
workspaces=1
extraction_runs=1
files=147
nodes=1337
edges=3674
occurrences=4158
edge_evidence=4158
```

## Visualize A Rust Workspace

The visualizer reads an existing SQLite graph and renders a bounded read-only
projection. The Rust backend defaults to `.local/rust-workspace-extract.db`,
which is the local fixture path used by the UI slice.

Create or refresh that fixture from the current workspace:

```sh
cargo run -p semantic-graph-extract -- rust-workspace-references \
  --db .local/rust-workspace-extract.db \
  --workspace-root .
```

Start the local JSON-RPC backend:

```sh
cargo run -p semantic-graph-visualizer-server -- \
  --database-path .local/rust-workspace-extract.db \
  --bind 127.0.0.1:5179
```

It serves `graph.projection`, `graph.node_details`, `graph.edge_details`, and
`graph.search_nodes` over JSON-RPC 2.0 at `http://127.0.0.1:5179/rpc`. The
default projection limit is 150 non-file symbols plus their file nodes and any
edges whose endpoints are both included in the projection.

Optional backend smoke request:

```sh
curl -sS -X POST http://127.0.0.1:5179/rpc \
  -H 'content-type: application/json' \
  --data '{"jsonrpc":"2.0","id":1,"method":"graph.projection","params":{"limit":150}}'
```

In another terminal, start the Blazor WebAssembly client:

```sh
ASPNETCORE_URLS=http://127.0.0.1:5180 \
  dotnet run --project apps/SemanticGraph.Visualizer/src/SemanticGraph.Visualizer.Client/SemanticGraph.Visualizer.Client.csproj \
  --no-launch-profile
```

Open `http://127.0.0.1:5180`. The client reads the backend base URL from
`apps/SemanticGraph.Visualizer/src/SemanticGraph.Visualizer.Client/wwwroot/appsettings.json`.
The toolbar search finds SQLite-backed graph nodes by name, qualified name, or
source path. Selecting visible nodes or edges loads details, occurrences, or
edge evidence into the right inspector.

## Start Fresh

Use this when you do not care about previous local smoke-test output and want a
clean database before extracting again. This is mostly useful while testing the
prototype, because it makes row counts easy to compare with the examples above.

Delete a disposable local DB and run extraction again:

```sh
rm -f .local/rust-extract-wip.db
cargo run -p semantic-graph-extract -- rust-document-symbols \
  --db .local/rust-extract-wip.db \
  --workspace-root . \
  --package-path crates/wip \
  --file crates/wip/src/lib.rs
```

The `.local/` directory is for local smoke-test output.

## Smoke Test

Run:

```sh
just rust-extract-smoke
```

That recipe extracts `crates/wip/src/lib.rs` into
`.local/rust-extract-wip.db` through the in-process `rust-analyzer-lib` route
and then prints store stats.

Crate and workspace smoke routes:

```sh
just rust-crate-extract-smoke
just rust-workspace-extract-smoke
just rust-workspace-references-smoke
```

The smoke-test crate also prints a route-level report:

```sh
SQLX_OFFLINE=true cargo run -p semantic-graph-smoke-tests
```

That report exercises the `rust-analyzer-lib` facade, the extractor crate route,
the extractor workspace route, and the workspace references route. Current
headline counts are:

```text
crate.persistence.files=4
crate.persistence.nodes=57
crate.persistence.edges=53
workspace.discovery.count=147
workspace.discovery.submodule_files=0
workspace.batch.files=147
workspace.batch.symbols=1190
workspace.persistence.files=147
workspace.persistence.nodes=1337
workspace.persistence.edges=1190
workspace.persistence.occurrences=1190
workspace.persistence.evidence=1190
workspace.references.targets=1153
workspace.references.edges=2484
workspace.references.occurrences=2968
workspace.references.file_fallbacks=647
workspace.references.skipped_external=0
workspace.references.persistence.files=147
workspace.references.persistence.nodes=1337
workspace.references.persistence.contains_edges=1190
workspace.references.persistence.references_edges=2484
workspace.references.persistence.reference_occurrences=2968
workspace.references.persistence.evidence=4158
workspace.references.persistence.routes_complete=148
workspace.references.persistence.stale_nodes_closed=0
workspace.references.persistence.stale_edges_closed=0
```

## Storage CLI

The storage CLI is useful for inspecting or demo-seeding a database, but it is
not required before extraction.

Initialize an empty DB manually:

```sh
cargo run -p semantic-graph-store -- init \
  --db .local/demo.db
```

Seed demo rows:

```sh
cargo run -p semantic-graph-store -- demo-seed \
  --db .local/demo.db \
  --root-uri file:///tmp/poc-semanticgraph
```

Print stats:

```sh
cargo run -p semantic-graph-store -- stats \
  --db .local/demo.db
```

## Confidence Check

Run the focused store/extract confidence path:

```sh
just confidence
```

This does not require a live `rust-analyzer` binary; extractor tests use
fixtures and the checked-in `rust-analyzer` libraries. It does not run the
smoke-test crate or the workspace extraction smoke route.

Useful focused checks:

```sh
SQLX_OFFLINE=true cargo check -p semantic-graph-extract
SQLX_OFFLINE=true cargo test -p semantic-graph-extract
SQLX_OFFLINE=true cargo clippy -p semantic-graph-extract --all-targets -- -D warnings
SQLX_OFFLINE=true cargo test -p semantic-graph-smoke-tests
SQLX_OFFLINE=true cargo run -p semantic-graph-smoke-tests
cargo check -p semantic-graph-visualizer-server
cargo test -p semantic-graph-visualizer-server
dotnet build SemanticGraph.Visualizer.slnx
```

## What Exists

- `crates/semantic-graph-store`: SQLite graph store and stats/demo CLI.
- `crates/semantic-graph-extract`: Rust document-symbol and workspace-reference
  extractor.
- `crates/semantic-graph-visualizer-server`: local read-only JSON-RPC backend
  for visualizer projection, search, and inspection.
- `crates/rust-analyzer-lib`: in-process facade over the pinned
  `rust-analyzer` submodule crates.
- `crates/semantic-graph-smoke-tests`: route smoke-test/report surface.
- `crates/wip`: small Rust crate used as the local extraction target.
- `apps/SemanticGraph.Visualizer`: Blazor WebAssembly, Radzen, and
  Blazor.Diagrams client for the read-only graph viewport and inspector.

The extractor currently writes:

- one `files` row for each extracted source file;
- one file node per extracted source file;
- symbol nodes from hierarchical Rust document-symbol data;
- `definition` occurrences for symbols;
- `contains` edges from file to top-level symbols and parent symbols to nested
  symbols;
- `references` edges from referencing symbols or fallback files to referenced
  symbols;
- reference occurrences for `textDocument/references` locations;
- edge evidence with `lsp_method = "textDocument/documentSymbol"` or
  `lsp_method = "textDocument/references"`;
- route status and route observations for document-symbol/reference freshness
  and stale closing.

## Not Yet Implemented

- First-class persisted crate/package rows.
- Call edges, dedicated definition edges, implementation/inheritance edges, or
  type hierarchy.
- C# extraction.
- CSV snapshots.
- Stale-row ownership policies for future semantic routes beyond document
  symbols and references.
- Full graph exploration UI beyond the bounded read-only projection, search,
  and inspector slice.

## Notes

- Rust extraction uses the checked-in `rust-analyzer` submodule crates through
  `rust-analyzer-lib`; the `rust-analyzer` binary is not required for live
  extraction commands or smoke recipes.
- The visualizer client uses Blazor.Diagrams and Radzen, with app behavior in
  Blazor/C# and no app-owned JavaScript.
- `lsp-types` is pinned to match
  `submodules/rust-analyzer/crates/rust-analyzer/Cargo.toml`.
- Submodules are local research inputs and should be treated as read-only unless
  a task explicitly asks to modify or update one.
