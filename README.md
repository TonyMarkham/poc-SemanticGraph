# SemanticGraph Prototype

SemanticGraph is a proof of concept for extracting code facts into a durable
SQLite graph.

Right now, the useful path is Rust single-file extraction through
`rust-analyzer`.

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

## Start Fresh

Use this when you do not care about previous local smoke-test output and want a
clean database before extracting again. This is mostly useful while testing the
prototype, because it makes row counts easy to compare with the examples above.

Delete the disposable local DB and run extraction again:

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
- `crates/semantic-graph-extract`: Rust single-file document-symbol extractor.
- `crates/wip`: small Rust crate used as the local extraction target.

The extractor currently writes:

- one `files` row for the extracted source file;
- one file node;
- symbol nodes from `textDocument/documentSymbol`;
- `definition` occurrences for symbols;
- `contains` edges from file to top-level symbols and parent symbols to nested
  symbols;
- edge evidence with `lsp_method = "textDocument/documentSymbol"`.

## Not Yet Implemented

- Whole-crate extraction.
- Calls, references, definitions, implementations, or type hierarchy.
- C# extraction.
- CSV snapshots.
- Stale-row handling.
- Graph visualization UI.

## Notes

- `rust-analyzer` must be installed only for live extraction commands and
  `just rust-extract-smoke`.
- `lsp-types` is pinned to match
  `submodules/rust-analyzer/crates/rust-analyzer/Cargo.toml`.
- Submodules are local research inputs and should be treated as read-only unless
  a task explicitly asks to modify or update one.
