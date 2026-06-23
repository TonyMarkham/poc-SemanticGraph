# SemanticGraph Prototype

SemanticGraph is a proof of concept for extracting code facts into a durable
SQLite graph.

Right now, the useful path is Rust single-file and workspace extraction through
the checked-in `rust-analyzer` libraries. The extractor supports full
single-file refresh, document-symbol-only, crate-scoped, workspace-scoped,
workspace-reference, workspace-call, and all-in-one workspace routes. A
read-only visualizer slice with projection, search, selection inspection, and
evidence display is available through a Rust JSON-RPC backend and a Blazor
WebAssembly client.

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

1. explicit CLI database path, such as `--db`, visualizer `--database-path`,
   or MCP server `--database-path`;
2. `--config <path>`;
3. discovered `.refactor-radar/config.toml`, searching upward from
   `--workspace-root` when a command has one, otherwise from the current
   directory;
4. an existing command default, where one exists.

Use `--db /tmp/scratch.db` for one-off extraction runs, benchmarks, or tests
that should ignore the configured database.

## Build The Release CLIs

Build the Rust command-line tools before running extraction, storage, MCP
server, or visualizer backend examples:

```sh
cargo build --release \
  -p semantic-graph-extract \
  -p semantic-graph-store \
  -p semantic-graph-mcp-server \
  -p semantic-graph-visualizer-server
```

Run the resulting binaries directly. This keeps long-running workspace
extraction easier to monitor and avoids the debug-profile overhead of
`cargo run`.

## Run The MCP Server

The MCP server reads an existing SemanticGraph SQLite database through the
same config discovery described above. Communication is always stdio, and the
server is read-only: it exposes graph query tools and resources but does not
run extractors or mutate the graph.

Start it from config:

```sh
./target/release/semantic-graph-mcp-server
```

Start it with a temporary database override:

```sh
./target/release/semantic-graph-mcp-server \
  --database-path .local/rust-workspace-extract.db
```

Example MCP-compatible host registration:

```toml
[mcp_servers.semantic_graph]
command = "/path/to/poc-SemanticGraph/target/release/semantic-graph-mcp-server"
args = []
```

Use `--database-path` in host config only when that launch should ignore
`.refactor-radar/config.toml` discovery:

```toml
[mcp_servers.semantic_graph]
command = "/path/to/poc-SemanticGraph/target/release/semantic-graph-mcp-server"
args = ["--database-path", "/path/to/poc-SemanticGraph/.local/rust-workspace-extract.db"]
```

These examples are host configuration snippets only; the server does not
install itself or generate Codex config.

## Extract One Rust File

Use this when you want to add or refresh facts for one file in a database. The
default `rust-file` workflow extracts symbols, references, and calls for that
file, then marks stale file-scoped observations that disappeared from the fresh
run. If the database already exists, the extractor reuses it and records new
extraction runs.

From the repo root:

```sh
./target/release/semantic-graph-extract rust-file crates/wip/src/lib.rs
```

or

```sh
./target/release/semantic-graph-extract rust-file \
  --db .local/rust-extract-wip.db \
  crates/wip/src/lib.rs
```

`rust-file` defaults `--workspace-root` to `.`, so pass
`--workspace-root <WORKSPACE_ROOT>` only when running from another directory or
when the extraction boundary should be explicit.

This creates or updates `.local/rust-extract-wip.db`.

You do not need to initialize the database first. The extractor opens the
SQLite database, runs migrations, loads the pinned `rust-analyzer` libraries
in-process through `rust-analyzer-lib`, starts one analysis worker for the
single-file command, extracts facts for the requested file, and writes the graph
rows.

Special route-only modes:

```sh
./target/release/semantic-graph-extract rust-file crates/wip/src/lib.rs --symbols
./target/release/semantic-graph-extract rust-file crates/wip/src/lib.rs --references
./target/release/semantic-graph-extract rust-file crates/wip/src/lib.rs --calls
```

`--symbols` refreshes only document symbols for the file. `--references` and
`--calls` refresh only those file-scoped relation routes and require the file's
symbol graph to already exist in the target database.

Example successful output:

```text
mode=full workspace=1 last_run=3 files=1 nodes=4 contains_edges=3 references_edges=2 reference_occurrences=2 calls_edges=0 call_occurrences=0 evidence=5 routes_complete=3 stale_nodes_closed=0 stale_edges_closed=0
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
extraction_runs=3
files=1
nodes=4
edges=5
occurrences=5
edge_evidence=5
```

## Extract A Different File

Use the same command and change the positional file path:

```sh
./target/release/semantic-graph-extract rust-file \
  --db .local/rust-extract-wip.db \
  crates/wip/src/models.rs
```

Each default `rust-file` invocation records document-symbol, reference, and call
runs. Canonical file, node, and edge rows are upserted, stale file-scoped facts
are soft-closed, and occurrence and evidence rows are inserted as run proof.

## Mark A Deleted Rust File Stale

Use this from file-watcher remove events when `notify` reports the path to a
Rust file that no longer exists:

```sh
./target/release/semantic-graph-extract rust-file-deleted crates/wip/src/foo.rs
```

`rust-file-deleted` defaults `--workspace-root` to `.` and accepts the deleted
file path even when it is already absent from disk. Relative deleted-file paths
are resolved under the workspace root.

The command writes through the DB manager, records completed file-scoped
document-symbol, reference, and call routes with zero observations, and marks
active graph facts for that file stale. This soft-closes nodes whose `file_id`
matches the deleted file and edges connected to those nodes or backed by
evidence/route observations from that file. It does not remove historical
occurrence/evidence rows or delete the `files` row.

Example successful output:

```text
mode=deleted file=crates/wip/src/foo.rs workspace=1 run=4 routes_complete=3 stale_nodes_closed=5 stale_edges_closed=4
```

## Extract A Rust Crate Or Workspace

The ergonomic crate and workspace commands use the same in-process
`rust-analyzer-lib` extraction path, with a single worker-pool knob:

```sh
./target/release/semantic-graph-extract rust-crate \
  --db .local/rust-crate-extract-wip.db \
  crates/wip
```

```sh
./target/release/semantic-graph-extract rust-workspace \
  --db .local/rust-workspace-extract.db
```

Both commands default `--workspace-root` to `.` and resolve
`--analysis-workers` from config when omitted. If no route selector is passed,
they extract document symbols, references, and calls. The route selectors are
combinable:

```sh
./target/release/semantic-graph-extract rust-crate crates/wip --symbols
./target/release/semantic-graph-extract rust-crate crates/wip --references
./target/release/semantic-graph-extract rust-workspace --symbols --references
./target/release/semantic-graph-extract rust-workspace --calls
```

`--symbols` refreshes document symbols for the selected crate or workspace.
`--references` and `--calls` refresh only relation routes and require the
symbol graph for the selected files to already exist in the target database
unless `--symbols` is selected in the same invocation.

`rust-workspace` hashes each discovered file before semantic extraction. When a
file's current hash matches a completed document-symbol route hash in the
database, the shared workspace runner loads that file's active symbols from
SQLite and skips further extraction work for that file. Default full runs and
`--symbols` both use this reuse path.

When changed files remain, `rust-workspace` loads the rust-analyzer workspace
once, creates per-worker `ide::Analysis` snapshots, persists changed document
symbols in one DB-manager batch, then extracts references and outgoing calls for
the changed origin files. Relation extraction still resolves against the active
symbol graph, including unchanged files loaded from SQLite. Fresh runs and other
runs where no files were skipped keep the original workspace relation batches;
partial incremental runs persist relation routes per processed origin file with
origin-file stale closing. Fresh database files skip document-symbol stale
closing because there is no prior graph state; existing database files keep
stale closing enabled. If every file is unchanged, the command skips starting
the shared rust-analyzer analysis pool.

The command prints the normal workspace summary with `scope=workspace` and
benchmark lines labeled `shared_analysis_snapshot` and `shared_workspace.*`.

Inspect the result:

```sh
./target/release/semantic-graph-store stats \
  --db .local/rust-crate-extract-wip.db
```

Expected shape:

```text
workspaces=1
extraction_runs=3
files=4
nodes=57
edges=<contains+references+calls>
occurrences=<definitions+references+calls>
edge_evidence=<definitions+references+calls>
```

Inspect a workspace extraction:

```sh
./target/release/semantic-graph-store stats \
  --db .local/rust-workspace-extract.db
```

Expected shape:

```text
workspaces=1
extraction_runs=3
files=<workspace_rust_files>
nodes=<files+symbols>
edges=<contains+references+calls>
occurrences=<definitions+references+calls>
edge_evidence=<definitions+references+calls>
```

## Extract C# Files, Projects, Or Solutions

C# extraction uses an installed `csharp-ls` process and persists graph rows under
the resolved solution's durable workspace identity. Resolve the solution with
`--solution <SLN_OR_SLNX>`, `[csharp].solution` in config, or by running from a
directory containing one `.slnx` or `.sln`.

Run the local fixture solution:

```sh
./target/release/semantic-graph-extract csharp-solution \
  --db .local/csharp-solution-extract.db \
  --solution __SmokeTestAssets__/csharp-wip/CSharpWip.sln
```

Run one C# file:

```sh
./target/release/semantic-graph-extract csharp-file \
  --db .local/csharp-file-extract.db \
  --solution __SmokeTestAssets__/csharp-wip/CSharpWip.sln \
  __SmokeTestAssets__/csharp-wip/Project/Worker.cs
```

Run one project boundary:

```sh
./target/release/semantic-graph-extract csharp-project \
  --db .local/csharp-project-extract.db \
  --solution __SmokeTestAssets__/csharp-wip/CSharpWip.sln \
  __SmokeTestAssets__/csharp-wip/Project/Project.csproj
```

`csharp-file` supports one route selector at a time:

```sh
./target/release/semantic-graph-extract csharp-file \
  --solution __SmokeTestAssets__/csharp-wip/CSharpWip.sln \
  --symbols \
  __SmokeTestAssets__/csharp-wip/Project/Worker.cs

./target/release/semantic-graph-extract csharp-file \
  --solution __SmokeTestAssets__/csharp-wip/CSharpWip.sln \
  --references \
  __SmokeTestAssets__/csharp-wip/Project/Worker.cs

./target/release/semantic-graph-extract csharp-file \
  --solution __SmokeTestAssets__/csharp-wip/CSharpWip.sln \
  --calls \
  __SmokeTestAssets__/csharp-wip/Project/Worker.cs
```

`csharp-project` and `csharp-solution` use combinable `--symbols`,
`--references`, and `--calls` selectors. Relation-only runs require the selected
files' symbol graph to already exist in the target database unless `--symbols`
is selected in the same invocation. Use `--process-workers <N>` to start more
than one `csharp-ls` worker process for project or solution batches.

`csharp-solution` hashes discovered files before starting `csharp-ls`. When a
file's hash matches a completed `csharp.document_symbols` file route in the
database, the command loads that file's active symbols from SQLite and skips
document-symbol extraction for that file. If every discovered file is unchanged,
default full runs and `--symbols` runs skip starting the C# worker pool.
References and calls are persisted per changed origin file; because the current
C# call route uses incoming call hierarchy, partial incremental relation passes
still query the active symbol graph as targets so calls from changed files to
unchanged symbols can be refreshed. Fresh solution runs and other runs where no
files were skipped keep the original solution relation batches.

Mark a removed C# file stale without starting `csharp-ls`:

```sh
./target/release/semantic-graph-extract csharp-file-deleted \
  --db .local/csharp-file-extract.db \
  --solution __SmokeTestAssets__/csharp-wip/CSharpWip.sln \
  Project/Worker.cs
```

The deleted-file path may be absolute, relative to the current directory, or
relative to the resolved solution directory. The command records completed
file-scoped `csharp.document_symbols`, `csharp.references`, and `csharp.calls`
routes with zero observations and soft-closes active graph facts for that file.

## Extract Soul Documents And Annotations

Soul extraction uses the checked-in Soul submodule in-process through
`soul-lsp-lib`. It live-scans the workspace with Soul's `indexer` APIs and the
`[soul]` section in `.refactor-radar/config.toml`. It does not run the
`soul-lsp` CLI and does not read or write Soul's `.soul/index.db`.

The extractor-owned Soul config mirrors Soul's scan settings and plugin list.
Plugins are required for code annotation files; without them Soul only sees
markdown documents and wikilinks.

```toml
[soul.scan]
excluded_dirs = [".git", ".soul", "target", ".idea", ".vscode", ".vs", ".codex", "node_modules", "obj"]
excluded_dir_suffixes = ["Tests", ".Tests", "tests", ".tests"]
excluded_bin_except_under = ["src"]

[[soul.plugins]]
language = "rust"
path = "./.soul/plugins/rust.so"

[[soul.plugins]]
language = "csharp"
path = "./.soul/plugins/csharp.so"
```

Run one Soul-backed file:

```sh
./target/release/semantic-graph-extract soul-file \
  --db .local/soul-extract.db \
  docs/feature.md
```

Run the Soul workspace:

```sh
./target/release/semantic-graph-extract soul-workspace \
  --db .local/soul-workspace-extract.db
```

`soul-file` supports one route selector at a time:

```sh
./target/release/semantic-graph-extract soul-file docs/feature.md --symbols
./target/release/semantic-graph-extract soul-file docs/feature.md --references
```

`soul-workspace` uses combinable `--symbols` and `--references` selectors. With
no selector, it refreshes `soul.document_symbols` and `soul.references`.
Relation-only `--references` runs require the Soul symbol graph to already exist
in the target database unless `--symbols` is selected in the same invocation.
Soul calls are not currently extracted because Soul LSP has no call hierarchy
route.

## Index File Text

Use `fts` to index workspace file contents without adding semantic nodes or
edges. SQLite stores file identity, document metadata, content hashes, and the
original UTF-8 text; Tantivy stores the derived search index in a sidecar
directory next to the database:

```sh
./target/release/semantic-graph-extract fts \
  --db .local/fts-content.db \
  --analysis-workers 8
```

The equivalent persistent settings live under `[fts]`:

```toml
[fts]
db_path = ".refactor-radar/fts.db"
analysis_workers = 8
max_indexed_file_bytes = 209715200
ignore-directories = [
    "target",
    "apps/SemanticGraph.Visualizer/src/SemanticGraph.Visualizer.Client/bin",
    "apps/SemanticGraph.Visualizer/src/SemanticGraph.Visualizer.Client/obj",
]
ignore-files = []
```

For `.local/fts-content.db`, the Tantivy index directory is
`.local/fts-content.tantivy`.

The command scans from the current directory, honors `[fts].ignore-directories`
and `[fts].ignore-files` in `.refactor-radar/config.toml`, hashes each
discovered file, skips rewriting unchanged documents, marks unchanged documents
seen in a single write batch, and closes removed documents at the end of a
successful run. File reads and hashes run through a worker pool; pass
`--analysis-workers N` to override `[fts].analysis_workers`, which falls back
to `[extractor].analysis_workers` when unset. Pass `--db` to override
`[fts].db_path`, which is resolved relative to the scanned workspace root and
falls back to `[database].path` when unset. Add `--no-rust`, `--no-csharp`, or
`--no-submodules` to exclude those file sets for the run.
`max_indexed_file_bytes` bounds the largest file read into the FTS index. When
the SQLite DB or Tantivy sidecar are inside the scanned
workspace, the route excludes its own artifacts from discovery. Tantivy remains
a rebuildable sidecar artifact; SQLite remains the source of truth for file
identity and stored content.

The stdio MCP server exposes indexed file-content search as the read-only
`fts_search` tool. It uses Tantivy for membership, ranking, and pagination, then
hydrates snippets from `fts_document_contents` in SQLite. The MCP server does
not run `semantic-graph-extract fts` or create missing FTS stores; run the
extractor first, then start the server with config discovery or
`--fts-database-path`. When `[fts].db_path` is unset, MCP uses the graph
database path as the FTS fallback and looks for the sidecar at
`db.with_extension("tantivy")`. Visualizer integration remains separate later
work.

## Visualize A Rust Workspace

The visualizer reads an existing SQLite graph and renders a bounded read-only
projection. The Rust backend resolves its database path from
`--database-path`, then `SEMANTIC_GRAPH_DB_PATH`, then config discovery, then
the legacy default `.local/rust-workspace-extract.db`.

Create or refresh that fixture from the current workspace:

```sh
./target/release/semantic-graph-extract rust-workspace
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
./target/release/semantic-graph-extract rust-file \
  --db .local/rust-extract-wip.db \
  crates/wip/src/lib.rs
```

The `.local/` directory is for local smoke-test output.

## Smoke Test

Run:

```sh
just rust-extract-smoke
```

That recipe extracts document symbols for `crates/wip/src/lib.rs` into
`.local/rust-extract-wip.db` through the in-process `rust-analyzer-lib` route
and then prints store stats. To smoke the default full single-file workflow
directly, run:

```sh
./target/release/semantic-graph-extract rust-file \
  --db /tmp/rust-file-scratch.db \
  crates/wip/src/lib.rs
```

Crate and workspace smoke routes:

```sh
just rust-crate-extract-smoke
just rust-workspace-extract-smoke
just rust-workspace-reference-route-smoke
just rust-workspace-call-route-smoke
just rust-workspace-smoke
```

The shared-vs-threaded workspace comparison is an ignored smoke test because it
runs both workspace implementations over the WIP crate:

```sh
SQLX_OFFLINE=true cargo test -p semantic-graph-smoke-tests \
  workspace_shared_matches_threaded_wip_counts -- --ignored --nocapture
```

C# smoke routes require `csharp-ls` on `PATH`:

```sh
just csharp-solution-smoke
just csharp-solution-reference-route-smoke
just csharp-solution-call-route-smoke
just csharp-file-smoke
just csharp-file-deleted-smoke
```

The smoke-test crate also prints a route-level report:

```sh
cargo build --release -p semantic-graph-smoke-tests
./target/release/semantic-graph-smoke-tests
```

That report exercises the `rust-analyzer-lib` facade, the extractor crate route,
the extractor workspace route, the workspace references route, and the
workspace calls route. It also exercises the local C# fixture through
`csharp-ls-lib`, including solution symbols, references, incoming calls, and
persistence. The full-workspace Rust references and calls unit tests and the
live `csharp-ls-lib` facade tests are ignored by default because they are
semantic smoke tests; run them explicitly when you need route confidence:

```sh
SQLX_OFFLINE=true cargo test -p semantic-graph-smoke-tests -- --ignored
SQLX_OFFLINE=true cargo test -p csharp-ls-lib -- --ignored
```

The report prints headline fields in this shape; exact counts depend on the
current Rust workspace contents, while the C# fixture counts should remain
stable:

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
workspace.shared_vs_threaded.threaded.files=<files>
workspace.shared_vs_threaded.shared.files=<files>
workspace.shared_vs_threaded.threaded.reference_edges=<references_edges>
workspace.shared_vs_threaded.shared.reference_edges=<references_edges>
workspace.shared_vs_threaded.threaded.call_edges=<calls_edges>
workspace.shared_vs_threaded.shared.call_edges=<calls_edges>
workspace.shared_vs_threaded.threaded.routes_complete=<threaded_route_completions>
workspace.shared_vs_threaded.shared.routes_complete=<shared_file_scoped_route_completions>
csharp.solution.discovery.count=1
csharp.solution.discovery.file=Project/Worker.cs
csharp.solution.symbols.files=1
csharp.solution.symbols.nodes=6
csharp.solution.references.targets=6
csharp.solution.references.edges=5
csharp.solution.references.occurrences=5
csharp.solution.references.file_fallbacks=0
csharp.solution.references.skipped_external=0
csharp.solution.calls.callable_nodes=3
csharp.solution.calls.edges=2
csharp.solution.calls.occurrences=2
csharp.solution.calls.skipped_external_targets=0
csharp.solution.calls.skipped_unresolved_targets=0
csharp.solution.calls.skipped_non_callable_prepare_items=0
csharp.solution.persistence.files=1
csharp.solution.persistence.nodes=7
csharp.solution.persistence.contains_edges=6
csharp.solution.persistence.occurrences=6
csharp.solution.persistence.evidence=6
csharp.solution.references.route.references_edges=5
csharp.solution.references.route.reference_occurrences=5
csharp.solution.references.route.routes_complete=1
csharp.solution.calls.route.calls_edges=2
csharp.solution.calls.route.call_occurrences=2
csharp.solution.calls.route.routes_complete=1
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
SQLX_OFFLINE=true cargo test -p csharp-ls-lib -- --ignored
cargo build --release -p semantic-graph-smoke-tests
./target/release/semantic-graph-smoke-tests
cargo check -p semantic-graph-visualizer-server
cargo test -p semantic-graph-visualizer-server
dotnet build SemanticGraph.Visualizer.slnx
```

## What Exists

- `crates/semantic-graph-store`: SQLite graph store and stats/demo CLI.
- `crates/semantic-graph-extract`: Rust and C# single-file, deleted-file,
  project/crate, workspace/solution, document-symbol, reference, call,
  file-content FTS, and all-in-one extractor.
- `crates/semantic-graph-visualizer-server`: local read-only JSON-RPC backend
  for visualizer projection, search, and inspection.
- `crates/rust-analyzer-lib`: in-process facade over the pinned
  `rust-analyzer` submodule crates.
- `crates/csharp-ls-lib`: process facade over an installed `csharp-ls` binary.
- `crates/semantic-graph-smoke-tests`: route smoke-test/report surface.
- `crates/wip`: small Rust crate used as the local extraction target.
- `__SmokeTestAssets__/csharp-wip`: small C# solution used as the local C#
  extraction target.
- `apps/SemanticGraph.Visualizer`: Blazor WebAssembly, Radzen, and
  Blazor.Diagrams client for the read-only graph viewport and inspector.

The extractor currently writes:

- one `files` row for each extracted source file;
- one file node per extracted source file;
- symbol nodes from hierarchical Rust and C# document-symbol data;
- `definition` occurrences for symbols;
- `contains` edges from file to top-level symbols and parent symbols to nested
  symbols;
- `references` edges from referencing symbols or fallback files to referenced
  symbols;
- reference occurrences for `textDocument/references` locations;
- `calls` edges from caller symbols to callee symbols;
- call occurrences for Rust `callHierarchy/outgoingCalls` ranges and C#
  `callHierarchy/incomingCalls` ranges mapped back to caller-to-callee edges;
- edge evidence with `lsp_method = "textDocument/documentSymbol"` or
  `lsp_method = "textDocument/references"` or
  `lsp_method = "callHierarchy/outgoingCalls"` or
  `lsp_method = "callHierarchy/incomingCalls"`;
- route status and route observations for document-symbol/reference/call
  freshness and stale closing.

## Not Yet Implemented

- First-class persisted crate/package rows.
- Dedicated definition edges, implementation/inheritance edges, or type
  hierarchy.
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
