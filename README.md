# SemanticGraph Prototype

SemanticGraph is a proof of concept for extracting code facts into a durable
SQLite graph.

Right now, the useful path is Rust document-symbol extraction through
`rust-analyzer`. The extractor supports single-file, crate-scoped, and
workspace-scoped routes.

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
SQLite database, runs migrations, starts `rust-analyzer`, extracts symbols for
the requested file, and writes the graph rows.

Example successful output:

```text
workspace=1 run=1 files=1 nodes=5 edges=4 occurrences=4 evidence=4
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
nodes=5
edges=4
occurrences=4
edge_evidence=4
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

The crate route uses `rust-analyzer lsif` for discovery, filters Rust document
vertices under `--package-path`, then extracts `textDocument/documentSymbol`
for each discovered file in one extraction run.

Example successful output for `crates/wip`:

```text
workspace=1 run=1 files=3 nodes=56 edges=53 occurrences=53 evidence=53
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
files=3
nodes=56
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

The workspace route uses the same LSIF-backed discovery path, but treats the
workspace root as the extraction boundary. In this repo, that excludes
`submodules/` because those crates are not part of the root Cargo workspace.

Example successful output for this repo:

```text
workspace=1 run=1 files=38 nodes=478 edges=440 occurrences=440 evidence=440
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
files=38
nodes=478
edges=440
occurrences=440
edge_evidence=440
```

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

If `rust-analyzer` is installed on `PATH`, run:

```sh
just rust-extract-smoke
```

That recipe extracts `crates/wip/src/lib.rs` into
`.local/rust-extract-wip.db` and then prints store stats.

Crate and workspace smoke routes:

```sh
just rust-crate-extract-smoke
just rust-workspace-extract-smoke
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

Run the repo confidence path:

```sh
just confidence
```

This does not require a live `rust-analyzer`; extractor tests use fixtures.

Useful focused checks:

```sh
SQLX_OFFLINE=true cargo check -p semantic-graph-extract
SQLX_OFFLINE=true cargo test -p semantic-graph-extract
SQLX_OFFLINE=true cargo clippy -p semantic-graph-extract --all-targets -- -D warnings
```

## What Exists

- `crates/semantic-graph-store`: SQLite graph store and stats/demo CLI.
- `crates/semantic-graph-extract`: Rust document-symbol extractor.
- `crates/wip`: small Rust crate used as the local extraction target.

The extractor currently writes:

- one `files` row for each extracted source file;
- one file node per extracted source file;
- symbol nodes from `textDocument/documentSymbol`;
- `definition` occurrences for symbols;
- `contains` edges from file to top-level symbols and parent symbols to nested
  symbols;
- edge evidence with `lsp_method = "textDocument/documentSymbol"`.

## Not Yet Implemented

- First-class persisted crate/package rows.
- A rust-analyzer library facade for discovery; crate/workspace discovery
  currently depends on `rust-analyzer lsif`.
- Calls, references, definitions, implementations, or type hierarchy.
- C# extraction.
- CSV snapshots.
- Stale-row handling.
- Graph visualization UI.

## Notes

- `rust-analyzer` must be installed for live extraction commands and smoke
  recipes.
- `lsp-types` is pinned to match
  `submodules/rust-analyzer/crates/rust-analyzer/Cargo.toml`.
- Submodules are local research inputs and should be treated as read-only unless
  a task explicitly asks to modify or update one.
