# VS-Config: Durable Refactor Radar Database Path

Status: Draft implementation plan
Date: 2026-06-14

## Goal

Introduce `.refactor-radar/config.toml` as the durable local configuration
file for the semantic graph database path.

The intended end state is:

- users do not need to pass `--db .local/rust-workspace-extract.db` to every
  command;
- extractor, store, visualizer, and MCP surfaces resolve the same database path
  consistently;
- CLI `--db` remains available as an explicit one-off override;
- the config format is small enough that later slices can extend it without
  changing how config discovery works.

## Non-Goals

- Do not define thread allocation settings in this slice.
- Do not define DB writer queue or batching settings in this slice.
- Do not define watcher settings in this slice.
- Do not define MCP behavior settings in this slice.
- Do not change extraction semantics, route freshness, or schema.
- Do not auto-rebuild the database from config loading.
- Do not use the installed `rust-analyzer` CLI.

## Config File

Create this file shape:

```toml
[database]
path = ".local/rust-workspace-extract.db"
```

Rules:

- `path` is required when the config file exists.
- Relative paths are resolved relative to the directory containing
  `.refactor-radar/config.toml`.
- Absolute paths are used as-is.
- The parent directory may be created by commands that create or migrate the
  database.
- Config loading must not create or migrate the database by itself.

## Discovery And Precedence

Use this precedence:

1. CLI `--db <path>`;
2. CLI `--config <path>` and that file's `[database].path`;
3. discovered `.refactor-radar/config.toml`;
4. existing command default, only for commands that already had one;
5. otherwise report a missing database path error.

Discovery rules:

- if `--config` is passed, use exactly that file;
- otherwise search upward for `.refactor-radar/config.toml` from
  `--workspace-root` when the command has one;
- otherwise search upward from the current working directory;
- stop at the filesystem root;
- do not search the user's home directory as a global fallback in this slice.

CLI `--db` must override the config path for benchmarking, scratch databases,
and tests.

## Shared Config API

Add a small shared Rust config surface rather than duplicating TOML parsing in
each binary.

Suggested crate:

```text
crates/semantic-graph-config
```

Suggested types:

```text
RefactorRadarConfig
DatabaseConfig
ConfigLoadOptions
ResolvedDatabasePath
```

Suggested functions:

```text
load_config(path) -> ConfigResult<RefactorRadarConfig>
discover_config(start_dir) -> ConfigResult<Option<PathBuf>>
resolve_database_path(options) -> ConfigResult<ResolvedDatabasePath>
```

Use the repo's typed error style. Do not add `anyhow`.

## CLI Work

Add a global optional `--config <path>` argument where practical.

For DB-backed commands, make `--db` optional only after config resolution is in
place.

Examples:

```sh
cargo run -p semantic-graph-extract -- rust-workspace-all \
  --workspace-root .
```

```sh
cargo run -p semantic-graph-extract -- rust-workspace-all \
  --config .refactor-radar/config.toml \
  --workspace-root .
```

```sh
cargo run -p semantic-graph-extract -- rust-workspace-all \
  --db /tmp/scratch.db \
  --workspace-root .
```

The first command uses discovered config. The third command explicitly
overrides config.

## Consumers

Apply config resolution to these surfaces:

- `semantic-graph-extract` commands that currently require `--db`;
- `semantic-graph-store` commands that currently require `--db`;
- visualizer server database-path default;
- future MCP server startup.

Do not require every consumer to be converted in one patch if that makes the
change too large. Start with `semantic-graph-extract rust-workspace-all` and
the store `stats` command, then finish the remaining DB-backed commands before
calling VS-Config complete.

## Tests

Config tests:

- parses a valid `.refactor-radar/config.toml`;
- rejects a config with missing `[database].path`;
- resolves relative database paths relative to the config file directory;
- preserves absolute database paths;
- discovers config from a workspace subdirectory;
- stops discovery at the filesystem root;
- CLI `--db` overrides config;
- missing config plus missing `--db` returns a typed error for commands without
  an existing default.

CLI tests:

- `rust-workspace-all --workspace-root .` uses the discovered DB path;
- `rust-workspace-all --db /tmp/scratch.db --workspace-root .` overrides the
  config path;
- `semantic-graph-store stats` can use the config path;
- command output reports the resolved database path when useful for debugging.

## Validation Commands

Expected validation after implementation:

```sh
SQLX_OFFLINE=true cargo test -p semantic-graph-config
SQLX_OFFLINE=true cargo test -p semantic-graph-extract
SQLX_OFFLINE=true cargo test -p semantic-graph-store
cargo run -p semantic-graph-extract -- rust-workspace-all \
  --workspace-root .
cargo run -p semantic-graph-store -- stats
```

If command examples change, update `README.md` in the same change.

Do not present the work as complete while `cargo check` or tests emit warnings.

## Acceptance Criteria

VS-Config is complete when all of the following are true:

- `.refactor-radar/config.toml` can define `[database].path`;
- DB-backed commands can resolve the configured database path;
- CLI `--db` overrides the configured path;
- relative paths resolve relative to the config file;
- config discovery works from workspace subdirectories;
- config loading uses typed errors;
- README documents the config file and override behavior;
- validation commands pass without warnings.

## Risks And Decisions To Preserve

- Keep this first config slice narrow.
- Do not add performance, watcher, or MCP settings here.
- Config loading should resolve paths, not mutate the graph.
- Explicit CLI overrides remain important for benchmarks and scratch runs.
