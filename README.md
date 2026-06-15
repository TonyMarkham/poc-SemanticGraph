# SemanticGraph Prototype

SemanticGraph is a proof of concept for extracting code facts into a durable
SQLite graph.

Right now, the useful path is Rust document-symbol extraction, workspace
reference extraction, and workspace call extraction through the checked-in
`rust-analyzer` libraries. The extractor supports single-file, crate-scoped,
workspace-scoped, workspace-reference, workspace-call, and all-in-one workspace
routes. A read-only visualizer slice with projection, search, selection
inspection, and evidence display is available through a Rust JSON-RPC backend
and a Blazor WebAssembly client.

## Configure The Local Database

DB-backed commands can read a local config file instead of requiring `--db` on
every invocation. The default discovered location is:

```toml
# .refactor-radar/config.toml
[database]
path = "content.db"

# Optional; defaults shown.
[writer]
queue_capacity = 4096
max_rows_per_commit = 1000
max_millis_per_commit = 250
busy_timeout_ms = 5000
```

When the config file exists, `[database].path` is required. Relative database
paths resolve relative to the directory containing the config file, so the
example above points at `.refactor-radar/content.db` from the repo root. An
absolute path is used as-is.

The optional `[writer]` section configures the queued SQLite writer used by
DB-backed write commands. Values must be greater than zero and are validated
before extraction starts.

Resolution precedence is:

1. explicit CLI database path, such as `--db` or visualizer `--database-path`;
2. `--config <path>`;
3. discovered `.refactor-radar/config.toml`, searching upward from
   `--workspace-root` when a command has one, otherwise from the current
   directory;
4. an existing command default, where one exists.

Use `--db /tmp/scratch.db` for one-off extraction runs, benchmarks, or tests
that should ignore the configured database.

## Build The Release CLIs

Build the Rust command-line tools before running extraction, storage, or
visualizer backend examples:

```sh
cargo build --release \
  -p semantic-graph-extract \
  -p semantic-graph-store \
  -p semantic-graph-visualizer-server
```

Run the resulting binaries directly. This keeps long-running workspace
extraction easier to monitor and avoids the debug-profile overhead of
`cargo run`.

## Extract One Rust File

Use this when you want to add or refresh facts for a file in a database. If the
database already exists, the extractor reuses it and records another extraction
run.

From the repo root:

```sh
./target/release/semantic-graph-extract rust-document-symbols \
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
./target/release/semantic-graph-store stats \
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
./target/release/semantic-graph-extract rust-document-symbols \
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
./target/release/semantic-graph-extract rust-crate-document-symbols \
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
./target/release/semantic-graph-store stats \
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
./target/release/semantic-graph-extract rust-workspace-document-symbols \
  --db .local/rust-workspace-extract.db \
  --workspace-root .
```

The workspace route uses the same `rust-analyzer-lib` source discovery path, but
treats the workspace root as the extraction boundary. In this repo, that
excludes `submodules/` because those crates are not part of the root Cargo
workspace.

Example successful output for the current repo workspace:

```text
workspace=1 run=1 files=172 nodes=1611 edges=1439 occurrences=1439 evidence=1439
```

Inspect the result:

```sh
./target/release/semantic-graph-store stats \
  --db .local/rust-workspace-extract.db
```

Expected shape:

```text
workspaces=1
extraction_runs=1
files=172
nodes=1611
edges=1439
occurrences=1439
edge_evidence=1439
```

## Extract Rust Workspace References

Use this after `rust-workspace-document-symbols` when you want to add or
refresh only Rust `references` edges in an existing workspace graph:

```sh
./target/release/semantic-graph-extract rust-workspace-document-symbols \
  --db .local/rust-workspace-references.db \
  --workspace-root .
```

```sh
./target/release/semantic-graph-extract rust-workspace-references \
  --db .local/rust-workspace-references.db \
  --workspace-root .
```

The references command requires the document-symbol graph to already exist in
the target database. It queries `rust-analyzer` references for eligible
workspace symbols, then stores directed `source --references--> target` edges
with occurrence and edge evidence proof.

Example successful output shape:

```text
workspace=1 run=2 targets=<targets> references_edges=3135 reference_occurrences=3829 evidence=3829 stale_edges_closed=0
```

Expected shape:

```text
workspaces=1
extraction_runs=2
files=172
nodes=1611
edges=4574
occurrences=5268
edge_evidence=5268
```

## Extract Rust Workspace Calls

Use this after `rust-workspace-document-symbols` when you want to add or
refresh only Rust `calls` edges in an existing workspace graph:

```sh
./target/release/semantic-graph-extract rust-workspace-document-symbols \
  --db .local/rust-workspace-calls.db \
  --workspace-root .
```

```sh
./target/release/semantic-graph-extract rust-workspace-calls \
  --db .local/rust-workspace-calls.db \
  --workspace-root .
```

The calls command requires the document-symbol graph to already exist in the
target database. It queries `rust-analyzer` outgoing call hierarchy for callable
workspace symbols, then stores directed `caller --calls--> callee` edges with
call occurrence and edge evidence proof. External and unresolved targets are
counted and skipped.

Example successful output shape:

```text
workspace=1 run=2 callable_nodes=<callable_nodes> calls_edges=243 call_occurrences=278 evidence=278 skipped_external_targets=<skipped_external_targets> skipped_unresolved_targets=<skipped_unresolved_targets> stale_edges_closed=0
```

Expected shape:

```text
workspaces=1
extraction_runs=2
files=172
nodes=1611
edges=1682
occurrences=1717
edge_evidence=1717
```

## Extract A Complete Rust Workspace

Use this when you want a fresh complete SQLite graph with document symbols,
references, and calls in one CLI invocation:

```sh
./target/release/semantic-graph-extract rust-workspace-all \
  --workspace-root .
```

The all-in-one command persists the document-symbol graph first, then refreshes
the references and calls routes in the same database.

To override the configured path for a one-off run:

```sh
./target/release/semantic-graph-extract rust-workspace-all \
  --db /tmp/rust-workspace-scratch.db \
  --workspace-root .
```

Example successful output for the current repo workspace:

```text
workspace=1 document_run=1 reference_run=2 call_run=3 files=172 nodes=1611 contains_edges=1439 references_edges=3135 reference_occurrences=3829 calls_edges=243 call_occurrences=278 evidence=5546 routes_complete=174 stale_nodes_closed=0 stale_edges_closed=0
```

Expected shape:

```text
workspaces=1
extraction_runs=3
files=172
nodes=1611
edges=4817
occurrences=5546
edge_evidence=5546
```

## Visualize A Rust Workspace

The visualizer reads an existing SQLite graph and renders a bounded read-only
projection. The Rust backend resolves its database path from
`--database-path`, then `SEMANTIC_GRAPH_DB_PATH`, then config discovery, then
the legacy default `.local/rust-workspace-extract.db`.

Create or refresh that fixture from the current workspace:

```sh
./target/release/semantic-graph-extract rust-workspace-all \
  --workspace-root .
```

Start the local JSON-RPC backend:

```sh
./target/release/semantic-graph-visualizer-server \
  --bind 127.0.0.1:5179
```

Pass `--database-path .local/rust-workspace-extract.db` when you want the
backend to ignore config discovery for that run.

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
./target/release/semantic-graph-extract rust-document-symbols \
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
just rust-workspace-calls-smoke
just rust-workspace-all-smoke
```

The smoke-test crate also prints a route-level report:

```sh
cargo build --release -p semantic-graph-smoke-tests
./target/release/semantic-graph-smoke-tests
```

That report exercises the `rust-analyzer-lib` facade, the extractor crate route,
the extractor workspace route, the workspace references route, and the
workspace calls route. The full-workspace references and calls unit tests are
ignored by default because they are expensive semantic smoke tests; run them
explicitly when you need route confidence:

```sh
SQLX_OFFLINE=true cargo test -p semantic-graph-smoke-tests -- --ignored
```

The report prints headline fields in this shape; exact counts depend on the
current workspace contents:

```text
crate.persistence.files=4
crate.persistence.nodes=57
crate.persistence.edges=53
workspace.discovery.count=<files>
workspace.discovery.submodule_files=0
workspace.batch.files=<files>
workspace.batch.symbols=<contains_edges>
workspace.persistence.files=<files>
workspace.persistence.nodes=<nodes>
workspace.persistence.edges=<contains_edges>
workspace.persistence.occurrences=<contains_edges>
workspace.persistence.evidence=<contains_edges>
workspace.references.targets=<targets>
workspace.references.edges=<references_edges>
workspace.references.occurrences=<reference_occurrences>
workspace.references.file_fallbacks=<file_fallbacks>
workspace.references.skipped_external=0
workspace.references.base.files=<files>
workspace.references.base.nodes=<nodes>
workspace.references.base.contains_edges=<contains_edges>
workspace.references.base.occurrences=<contains_edges>
workspace.references.base.evidence=<contains_edges>
workspace.references.route.files=0
workspace.references.route.nodes=0
workspace.references.route.contains_edges=0
workspace.references.route.references_edges=<references_edges>
workspace.references.route.reference_occurrences=<reference_occurrences>
workspace.references.route.evidence=<reference_occurrences>
workspace.references.route.routes_complete=1
workspace.references.route.stale_nodes_closed=0
workspace.references.route.stale_edges_closed=0
workspace.calls.callable_nodes=<callable_nodes>
workspace.calls.edges=<calls_edges>
workspace.calls.occurrences=<call_occurrences>
workspace.calls.skipped_external_targets=<skipped_external_targets>
workspace.calls.skipped_unresolved_targets=<skipped_unresolved_targets>
workspace.calls.base.files=<files>
workspace.calls.base.nodes=<nodes>
workspace.calls.base.contains_edges=<contains_edges>
workspace.calls.base.occurrences=<contains_edges>
workspace.calls.base.evidence=<contains_edges>
workspace.calls.route.files=0
workspace.calls.route.nodes=0
workspace.calls.route.contains_edges=0
workspace.calls.route.calls_edges=<calls_edges>
workspace.calls.route.call_occurrences=<call_occurrences>
workspace.calls.route.evidence=<call_occurrences>
workspace.calls.route.routes_complete=1
workspace.calls.route.stale_nodes_closed=0
workspace.calls.route.stale_edges_closed=0
```

## Storage CLI

The storage CLI is useful for inspecting or demo-seeding a database, but it is
not required before extraction.

Initialize an empty DB manually:

```sh
./target/release/semantic-graph-store init \
  --db .local/demo.db
```

Seed demo rows:

```sh
./target/release/semantic-graph-store demo-seed \
  --db .local/demo.db \
  --root-uri file:///tmp/poc-semanticgraph
```

Print stats:

```sh
./target/release/semantic-graph-store stats
```

Pass `--db .local/demo.db` to inspect a specific database instead of the
configured one.

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
cargo build --release -p semantic-graph-smoke-tests
./target/release/semantic-graph-smoke-tests
cargo check -p semantic-graph-visualizer-server
cargo test -p semantic-graph-visualizer-server
dotnet build SemanticGraph.Visualizer.slnx
```

## What Exists

- `crates/semantic-graph-store`: SQLite graph store and stats/demo CLI.
- `crates/semantic-graph-extract`: Rust document-symbol, workspace-reference,
  and workspace-call extractor.
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
- `calls` edges from caller symbols to callee symbols;
- call occurrences for `callHierarchy/outgoingCalls` callsite ranges;
- edge evidence with `lsp_method = "textDocument/documentSymbol"` or
  `lsp_method = "textDocument/references"` or
  `lsp_method = "callHierarchy/outgoingCalls"`;
- route status and route observations for document-symbol/reference/call
  freshness and stale closing.

## Not Yet Implemented

- First-class persisted crate/package rows.
- Dedicated definition edges, implementation/inheritance edges, or type
  hierarchy.
- C# extraction.
- CSV snapshots.
- Stale-row ownership policies for future semantic routes beyond document
  symbols, references, and calls.
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
