# VS-DbWriteManager: Queued SQLite Write Manager

Status: Draft implementation plan
Date: 2026-06-14

## Goal

Add a route-aware SQLite write manager so extraction workers can emit typed
semantic graph batches without competing for direct database writes.

The intended end state is:

- SQLite has one owned writer path during extraction;
- extraction workers send typed batches through a bounded queue;
- writes are grouped into explicit transactions;
- route status, observations, evidence, occurrences, and stale closing remain
  centrally ordered;
- progress reporting can distinguish extracted, queued, written, committed,
  and stale-closed work;
- serial extraction and future threaded extraction produce the same graph.

## Motivation

SQLite WAL mode improves reader/writer behavior, but it does not make SQLite a
multi-writer database. Concurrent writers still serialize and can hit busy
states. A future threaded extractor should not have each worker open its own
write transaction and hope SQLite scheduling produces predictable behavior.

This repo's persistence is not just "insert rows". It has semantic ordering:

- start extraction runs;
- start and complete route statuses;
- upsert current nodes and edges;
- insert occurrences and evidence;
- record route observations;
- close stale nodes and edges only after successful route completion;
- avoid stale closing when a route fails.

That ordering belongs in one owned writer component.

## Non-Goals

- Do not expose arbitrary SQL write jobs.
- Do not remove the current synchronous persister in the first patch unless the
  manager has parity tests.
- Do not change graph semantics or route names.
- Do not make SQLite the extractor work scheduler.
- Do not add watcher behavior in this plan.
- Do not add threaded rust-analyzer workers in this plan.
- Do not use the installed `rust-analyzer` CLI.

## Current Evidence

The current store already has the primitives a write manager must call:

- `GraphStore::connect` and migrations;
- extraction run start/finish methods;
- file, node, edge upsert methods;
- occurrence and edge evidence insert methods;
- route status start/complete/fail methods;
- route observation recording;
- stale node and edge closing by route.

The current extractor persists document symbols, references, and calls through
`ExtractionPersister`. That code is the semantic source of truth for write
ordering and should be reused or carefully split, not bypassed.

## Dependency On VS-Config

VS-DbWriteManager depends on VS-Config. The writer should receive a database
path that has already been resolved from `.refactor-radar/config.toml` or a
CLI `--db` override.

The write manager should not implement its own config discovery.

## Design Summary

Introduce a single writer task for extraction runs.

Workers produce typed write batches:

```text
DocumentSymbolWriteBatch
ReferenceWriteBatch
CallWriteBatch
OccurrenceWriteBatch
EvidenceWriteBatch
RouteObservationWriteBatch
RouteLifecycleCommand
RunLifecycleCommand
```

The writer receives batches over a bounded channel, applies them through
`GraphStore`, commits in controlled chunks, and emits progress events.

The first implementation can run in-process without separate OS threads. The
important boundary is ownership: only the write manager performs SQLite writes
for a managed extraction run.

## Writer Ownership Rules

The write manager owns:

- the write connection or pool;
- the active extraction run IDs it started;
- route lifecycle state;
- transaction boundaries;
- batch ordering;
- stale closing;
- write progress counters;
- failure finalization.

Extraction workers own:

- rust-analyzer analysis queries;
- target/callable job execution;
- mapping provider results to typed graph facts;
- sending complete, typed batches;
- stopping promptly when cancellation is requested.

Workers must not:

- call `GraphStore` write methods directly during managed extraction;
- record route status;
- close stale rows;
- decide that a failed route may stale-close old facts.

## Batch Types

Keep batches typed and narrow. Do not send raw SQL.

Suggested high-level commands:

```text
BeginRun
FinishRun
FailRun
BeginRoute
CompleteRoute
FailRoute
UpsertFiles
UpsertNodes
UpsertEdges
InsertOccurrences
InsertEdgeEvidence
RecordRouteObservations
CloseStaleForRoute
Flush
Shutdown
```

Each command should include enough route/run context to validate ordering.

Recommended shared metadata:

```text
workspace_root_uri
workspace_id
run_id
route
scope
scope_key
provider
provider_version
source_file_id when known
```

## Route Ordering Policy

The write manager must enforce route-safe ordering.

For document symbols:

```text
BeginRun
BeginRoute rust.document_symbols for file
UpsertFiles
UpsertNodes
UpsertContainsEdges
InsertDefinitionOccurrences
InsertDocumentSymbolEvidence
RecordRouteObservations
CompleteRoute
CloseStaleNodesForRoute
CloseStaleEdgesForRoute
FinishRun
```

For references:

```text
BeginRun
BeginRoute rust.references workspace
UpsertReferenceEdges
InsertReferenceOccurrences
InsertReferenceEvidence
RecordRouteObservations
CompleteRoute
CloseStaleEdgesForRoute
FinishRun
```

For calls:

```text
BeginRun
BeginRoute rust.calls workspace
UpsertCallEdges
InsertCallOccurrences
InsertCallEvidence
RecordRouteObservations
CompleteRoute
CloseStaleEdgesForRoute
FinishRun
```

If a route fails:

- mark the route failed;
- mark the run failed if appropriate;
- do not close stale rows for that route.

## Transaction Strategy

Use explicit transactions around meaningful chunks.

Initial defaults:

```text
max_rows_per_commit = 1000
max_millis_per_commit = 250
busy_timeout_ms = 5000
journal_mode = WAL
synchronous = NORMAL
```

These values are starting points, not hard requirements. They should be
measured against the smoke workspace and the real workspace.

Rules:

- never commit half of a single canonical edge plus its required route
  observation if that would make stale closing incorrect;
- keep run and route lifecycle commands outside ambiguous batch boundaries;
- expose failed commit errors immediately;
- prefer deterministic order inside each batch before writing.

## Config Settings

VS-DbWriteManager may extend `.refactor-radar/config.toml` with writer-specific
settings:

```toml
[writer]
queue_capacity = 4096
max_rows_per_commit = 1000
max_millis_per_commit = 250
busy_timeout_ms = 5000
journal_mode = "wal"
synchronous = "normal"
```

Rules:

- `[database].path` remains owned by VS-Config;
- CLI writer flags override `[writer]` values when present;
- writer settings should have conservative built-in defaults;
- invalid writer settings should produce typed config errors before extraction
  starts.

## Backpressure

Use bounded queues.

Backpressure is a feature here. If workers can produce evidence faster than
SQLite can write it, unbounded queues will turn memory into the bottleneck.

Recommended knobs:

```text
write_queue_capacity
max_batch_rows
max_pending_bytes optional later
```

Progress should include queue depth so slow write phases are visible.

## Progress Events

Add a small progress model that can be used by CLIs, future MCP tools, and a
future watcher.

Suggested event types:

```text
RunStarted
RouteStarted
BatchQueued
BatchWriting
BatchCommitted
RouteCompleted
RouteFailed
StaleClosed
RunCompleted
RunFailed
WriterBackpressure
```

Each event should be machine-readable and also easy to render as CLI text.

## API Shape

Add a focused module, likely in `semantic-graph-extract` first:

```text
write_manager/
  mod.rs
  db_write_manager.rs
  db_write_command.rs
  db_write_progress.rs
  db_write_config.rs
  db_write_error.rs
```

If it grows beyond extractor ownership, move reusable pieces into
`semantic-graph-store`.

Suggested interface:

```text
DbWriteManager::start(store, config) -> DbWriteHandle
DbWriteHandle::send(command) -> ExtractResult<()>
DbWriteHandle::flush() -> ExtractResult<()>
DbWriteHandle::shutdown() -> ExtractResult<DbWriteSummary>
DbWriteHandle::subscribe_progress() -> ProgressReceiver
```

The first version can be async-only because the current store APIs are async.

## Integration Strategy

Do this in two passes.

Pass 1: compatibility mode.

- Add the write manager.
- Read writer settings through the VS-Config config model.
- Route existing serial persistence through it for one route at a time.
- Preserve current CLI outputs.
- Compare DB output to the existing persister.

Pass 2: managed extraction mode.

- Make `rust-workspace-all` use the write manager by default.
- Keep a temporary serial fallback flag until parity is proven.
- Remove the fallback only after tests and smoke output are stable.

## Tests

Store/write-manager tests:

- preserves current document-symbol persistence output;
- preserves current reference persistence output;
- preserves current call persistence output;
- commits batches under the configured row threshold;
- flushes on shutdown;
- rejects invalid route ordering;
- marks failed routes without stale closing;
- closes stale rows only after completed routes;
- applies deterministic ordering for repeated runs;
- reports progress events in route order;
- backpressure blocks producers instead of growing unbounded memory.

Parity tests:

- run current serial persistence and write-manager persistence against the same
  extraction fixture;
- compare canonical current rows for files, nodes, edges, occurrences,
  evidence counts, route statuses, and route observations;
- compare stale closing behavior after removing a symbol/reference/call.

## Validation Commands

Expected validation after implementation:

```sh
SQLX_OFFLINE=true cargo test -p semantic-graph-store
SQLX_OFFLINE=true cargo test -p semantic-graph-extract
SQLX_OFFLINE=true cargo test -p semantic-graph-smoke-tests
SQLX_OFFLINE=true cargo run -p semantic-graph-smoke-tests
cargo run -p semantic-graph-extract -- rust-workspace-all \
  --workspace-root .
cargo run -p semantic-graph-store -- stats
```

If smoke counts change because duplicate writes or stale behavior changed,
investigate before updating `README.md`.

Do not present the work as complete while `cargo check`, tests, or smoke runs
emit warnings.

## Acceptance Criteria

VS-DbWriteManager is complete when all of the following are true:

- a single managed writer can persist document symbols, references, and calls;
- writer settings are loaded through the VS-Config config model;
- workers can send typed write batches without direct SQLite writes;
- the writer batches commits and exposes progress events;
- route lifecycle and stale closing remain centralized and correct;
- failed routes do not close stale rows;
- output matches the existing serial persister for the smoke fixtures;
- `rust-workspace-all` can run through the write manager;
- validation commands pass without warnings.

## Risks And Decisions To Preserve

- Do not turn the writer queue into arbitrary SQL execution.
- Do not optimize by weakening evidence or route observations.
- WAL helps readers, but one writer still owns write correctness.
- A busy timeout is useful, but avoiding write contention is the real design.
- Deterministic output matters because future threaded extraction will otherwise
  create hard-to-debug graph churn.
