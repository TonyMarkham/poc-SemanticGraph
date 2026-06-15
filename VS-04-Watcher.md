# VS-Watcher: Incremental File Watcher And Refresh Daemon

Status: Draft implementation plan
Date: 2026-06-14

## Goal

Add a local watcher mode that observes workspace changes and refreshes the
semantic graph incrementally where the route model can do so correctly.

The intended end state is:

- file changes enqueue targeted refresh work;
- document-symbol updates are refreshed at file scope;
- relation refresh work is limited when safe and escalates to broader refresh
  when required;
- updates flow through the DB write manager;
- progress and status are visible to CLI and future MCP tools;
- the watcher never corrupts route freshness by pretending a partial refresh is
  a complete workspace route.

## Dependencies

VS-Watcher should come after:

- VS-Config, so watcher mode uses the same durable database path and config
  discovery as the extractor;
- VS-DbWriteManager, so background refreshes have one safe write path;
- VS-Threaded, so queued refresh jobs can use worker pools without blocking the
  watcher loop.

An MVP document-symbol-only watcher can be built earlier, but references and
calls require a correct affected-set and stale policy before they should run
incrementally.

## Non-Goals

- Do not implement an IDE.
- Do not watch every file type in the repo.
- Do not index comments or raw full text.
- Do not silently run expensive full workspace refreshes without reporting
  why.
- Do not mark workspace-scoped reference or call routes complete after only a
  partial refresh.
- Do not use the installed `rust-analyzer` CLI.

## Current Evidence

The current graph already supports parts of incremental behavior:

- `rust.document_symbols` is file-scoped;
- stale document-symbol nodes and `contains` edges can be closed per file
  route;
- `rust.references` and `rust.calls` are workspace-scoped today;
- route observations include `source_file_id`, which can support later
  affected-source-file policies;
- `rust-workspace-all` can rebuild the whole DB when uncertainty is too broad.

The key constraint is route truthfulness. A changed-file refresh must not claim
that the entire workspace references or calls route is current unless the
entire route was actually refreshed.

## Watch Scope

Watch these paths first:

```text
*.rs
Cargo.toml
Cargo.lock
build.rs
rust-toolchain
rust-toolchain.toml
```

Ignore:

```text
target/
.git/
.local/
submodules/ target directories
editor swap/temp files
```

Treat submodules as read-only research corpus unless the user explicitly
starts a watcher for a submodule path.

## Change Classification

Classify events before enqueueing extraction.

Suggested classes:

```text
RustSourceChanged
RustSourceDeleted
RustSourceCreated
CargoMetadataChanged
BuildScriptChanged
ToolchainChanged
UnknownRelevantChange
IrrelevantChange
```

Default policy:

- Rust source changes can refresh document symbols for affected files.
- Cargo metadata, build script, toolchain, or unknown relevant changes should
  mark relation routes stale or schedule a full workspace refresh.
- Deletes should close document-symbol-owned nodes and `contains` edges for
  that file route.

## Debounce Policy

Use debounce windows to avoid thrashing during saves, branch switches, or code
generation.

Suggested defaults:

```text
debounce_ms = 300
max_debounce_ms = 2000
branch_switch_quiet_ms = 3000
```

Batch events by workspace and change class. Emit progress saying what was
coalesced.

## Config Settings

VS-Watcher may extend `.refactor-radar/config.toml` with watcher-specific
settings:

```toml
[watcher]
enabled = false
full_on_start = false
doc_symbols_only = false
debounce_ms = 300
max_debounce_ms = 2000
branch_switch_quiet_ms = 3000
max_changed_files = 200
```

Rules:

- `[database].path` remains owned by VS-Config;
- watcher CLI flags override `[watcher]` values;
- watcher settings must not change extraction semantics by themselves;
- `enabled = true` should not start a watcher unless the selected command or
  runtime surface is watcher-capable.

## Incremental Levels

Implement incremental behavior in levels.

### Level 1: Document Symbols Only

For changed `.rs` files:

- run file-scoped document-symbol extraction;
- persist through the write manager;
- close stale nodes and `contains` edges owned by that file route;
- mark dependent relation routes as needing refresh if symbols changed.

This is safe with the current route model.

### Level 2: Changed Source Relation Contributions

Add a new route model for source-file-scoped relation contributions before
using this level.

Possible route names:

```text
rust.references.source_file
rust.calls.source_file
```

These routes would own only edges/occurrences/evidence caused by a specific
source file's outgoing references or calls. They must not close facts owned by
the existing workspace routes.

This requires explicit design because current `rust.references` and
`rust.calls` are workspace-scoped.

### Level 3: Affected Symbol Refresh

Use the graph to identify symbols affected by changed files:

- symbols defined in changed files;
- callers in changed files;
- reference sources in changed files;
- symbols whose definitions moved or disappeared;
- dependent relation edges from previous observations.

Refresh relation jobs for that affected set only when route ownership can
close stale facts correctly.

### Level 4: Full Workspace Refresh

Fall back to `rust-workspace-all` semantics when incremental safety is unclear.

Examples:

- `Cargo.toml` changed;
- `Cargo.lock` changed;
- `build.rs` changed;
- proc-macro behavior may have changed;
- large branch switch detected;
- too many files changed;
- watcher missed events;
- rust-analyzer analysis reports broad invalidation.

## Route Freshness Policy

The watcher must preserve route honesty.

Rules:

- file-scoped document-symbol routes can be completed for individual files;
- workspace-scoped routes can only be completed by full workspace refresh;
- partial relation routes need their own route names/scopes;
- failed refreshes must not stale-close existing facts;
- when a document-symbol change invalidates relation facts but relation refresh
  is not run, route status should report that relation data may be stale.

If the current schema cannot represent "dependent route needs refresh", add a
small migration rather than hiding that state in logs.

## CLI Shape

Add a watcher command after the write manager exists.

Suggested command:

```sh
cargo run -p semantic-graph-extract -- rust-workspace-watch \
  --workspace-root .
```

Suggested flags:

```text
--full-on-start
--doc-symbols-only
--jobs <n>
--debounce-ms <n>
--max-changed-files <n>
--once
```

`--once` is useful for tests: collect current git/worktree changes, process
them once, then exit.

## Git Integration

File watcher events should be the live path. Git can provide a useful startup
or recovery path.

Suggested commands to support internally or document:

```text
git status --porcelain
git diff --name-only
git diff --name-only --cached
git diff --name-only HEAD
```

Use git data to:

- seed initial changed-file work;
- recover after watcher overflow;
- detect branch-switch scale changes;
- avoid scanning unrelated files.

Git status should not be the only source of truth because unsaved editor
buffers and filesystem events can be more current than Git.

## Progress And Status

Expose watcher status as structured events.

Suggested event types:

```text
WatcherStarted
ChangeBatchDetected
ChangeBatchClassified
RefreshQueued
RefreshStarted
RefreshCompleted
RefreshFailed
EscalatedToFullRefresh
RoutesMarkedStale
WatcherIdle
WatcherStopped
```

Status fields:

```text
pending_files
pending_relation_jobs
active_workers
write_queue_depth
last_successful_refresh
last_failed_refresh
routes_current
routes_stale_or_unknown
```

VS-30 MCP tools should eventually expose this status so Codex can know whether
the database is current before answering semantic questions.

## Failure Policy

Watcher failures must be visible and conservative.

Rules:

- missed events trigger a full refresh or mark routes stale;
- failed document-symbol refresh marks only that file route failed;
- failed relation refresh marks the relevant relation route failed;
- failed routes do not close stale facts;
- repeated failures should stop automatic retry after a small limit and report
  a blocked watcher state;
- database lock or write-manager failure should stop the watcher rather than
  continue with false freshness.

## Tests

Watcher tests:

- debounce multiple saves into one refresh batch;
- ignore target, `.git`, `.local`, and temp files;
- classify `.rs` changes as document-symbol refresh work;
- classify Cargo metadata changes as full-refresh or broad-stale work;
- update one changed file without touching unrelated file routes;
- close stale document-symbol nodes after a deleted symbol;
- do not mark workspace references/calls complete after doc-symbol-only
  refresh;
- trigger full refresh after too many changed files;
- recover from simulated watcher overflow by using git/workspace scan;
- expose accurate idle/current/stale status.

Integration tests:

- start from a complete workspace DB;
- edit a fixture Rust file;
- run watcher `--once`;
- verify updated document symbols and stale closing;
- verify relation routes are either refreshed by a supported incremental route
  or explicitly marked stale/unknown.

## Validation Commands

Expected validation after implementation:

```sh
SQLX_OFFLINE=true cargo test -p semantic-graph-extract
SQLX_OFFLINE=true cargo test -p semantic-graph-store
SQLX_OFFLINE=true cargo test -p semantic-graph-smoke-tests
cargo run -p semantic-graph-extract -- rust-workspace-all \
  --workspace-root .
cargo run -p semantic-graph-extract -- rust-workspace-watch \
  --workspace-root . \
  --once
```

If watcher status is surfaced through MCP, also validate the VS-30 status tool.

Do not present the work as complete while `cargo check`, tests, watcher startup,
or smoke runs emit warnings.

## Acceptance Criteria

VS-Watcher is complete when all of the following are true:

- watcher mode observes relevant workspace changes;
- watcher settings can be loaded from `.refactor-radar/config.toml`;
- events are debounced and classified;
- changed Rust files refresh file-scoped document-symbol routes safely;
- relation routes are refreshed only when their route scope can be made true;
- otherwise relation routes are marked stale/unknown or a full refresh is
  scheduled;
- writes go through the DB write manager;
- progress/status reports pending, active, completed, failed, and stale state;
- missed events and broad project changes trigger conservative recovery;
- watcher `--once` can be used in tests;
- validation commands pass without warnings.

## Risks And Decisions To Preserve

- A watcher without correct route freshness is worse than a slower batch tool.
- Workspace-scoped references and calls cannot honestly be partially completed.
- File events are noisy; debounce and classification are required.
- Cargo metadata and proc-macro changes can invalidate more than the changed
  file.
- Do not let convenience turn into silent false freshness.
