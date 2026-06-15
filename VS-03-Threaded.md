# VS-Threaded: Parallel Rust Extraction Pipeline

Status: Draft implementation plan
Date: 2026-06-14

## Goal

Parallelize Rust reference and call extraction after document symbols are
current, while preserving the same durable graph semantics as the serial
extractor.

The intended end state is:

- one workspace analysis load is shared where the pinned rust-analyzer API
  allows it;
- document symbols are refreshed before relation work starts;
- reference target jobs run on a bounded worker pool;
- call hierarchy jobs run on a bounded worker pool;
- writes flow through the DB write manager;
- route completion and stale closing happen only after all jobs for that route
  succeed;
- threaded and serial extraction produce equivalent current graph facts.

## Dependencies

VS-Threaded depends on:

- VS-Config, so worker counts and mode can be configured in the same durable
  config file as the database path;
- VS-DbWriteManager, unless a source spike proves that direct concurrent writes
  are simpler and equally safe.

The desired architecture is:

```text
shared rust-analyzer analysis
  -> document-symbol refresh
  -> reference workers
  -> call workers
  -> typed write batches
  -> single DB write manager
  -> SQLite
```

Without a single writer, threaded extraction can easily spend its speedup on
SQLite contention and route-ordering bugs.

## Non-Goals

- Do not add watcher behavior.
- Do not add new semantic relation families.
- Do not weaken route freshness, evidence, or stale closing.
- Do not use installed `rust-analyzer` CLI or a rust-analyzer process.
- Do not assume rust-analyzer analysis APIs are thread-safe without verifying
  the pinned source and compiling the implementation.
- Do not create one independent rust-analyzer workspace load per worker unless
  the source spike proves shared analysis is impossible and benchmarks prove
  multiple loads are still worthwhile.
- Do not make output nondeterministic.

## Current Evidence

The current extractor already has route separation:

- document symbols;
- references;
- calls;
- all-in-one workspace command.

The current implementation also has a likely inefficiency: references and calls
are extracted separately and can reload or re-walk analysis state. Both relation
routes are naturally job-shaped after document symbols exist:

- references query one target symbol at a time;
- calls query one callable symbol at a time.

That makes them good candidates for bounded worker pools.

The desired performance win is not multiple copies of `rust-analyzer-lib`.
Repeatedly loading the same workspace would duplicate memory, caches, Cargo
metadata, proc-macro setup, and analysis work. The preferred design is one
loaded analysis context with bounded worker pools querying shared or cheaply
cloned analysis handles.

## Source Spike

Start with a focused rust-analyzer facade spike.

The spike must answer:

- whether the loaded `Analysis` or snapshot type can be shared across threads;
- whether cloning `Analysis` is cheap and shares the same underlying analysis
  database;
- whether individual reference and call queries can run concurrently;
- whether the API internally serializes expensive work anyway;
- whether shared analysis must be cloned per worker or borrowed behind an
  `Arc`;
- whether a single analysis-owner worker thread is needed if the analysis type
  is not `Send + Sync`;
- whether cancellation is available or must be cooperative at job boundaries;
- whether parallel queries produce stable provider results;
- where memory growth appears under worker pressure;
- what memory and wall-clock cost would come from multiple independent
  analysis loads if that fallback is considered.

Do not proceed with broad threading until this is answered from the pinned
submodule and a compiling prototype.

## Analysis Sharing Policy

Default to one rust-analyzer workspace load per extraction run.

Preferred order:

1. Share one loaded analysis context across worker pools if the pinned API is
   safe and efficient for concurrent queries.
2. Use cheap cloned analysis snapshots per worker only if clones share the same
   underlying database/cache.
3. Use one analysis-owning worker thread that receives reference and call jobs
   if the analysis object cannot be shared across threads.
4. Consider a small number of independent analysis loads only as a measured
   fallback, with explicit memory limits and serial-parity tests.

The implementation should fail back to serial querying before it silently
spawns many full workspace analysis instances.

## Provider Execution Handshake

Do not hard-code Rust threading behavior into the core extractor. Add an
explicit provider/runtime handshake so future providers, especially C#, can
declare how each semantic route may be executed.

Semantic route identity must stay separate from execution mode:

```text
route = "rust.references"
execution_mode = "parallel_shared_analysis"
```

Do not create route names like `rust.references.parallel` or
`rust.references.serial`. Stale closing, route status, and observations belong
to the semantic route. Execution mode belongs in route diagnostics,
properties, or run metadata.

Suggested provider capability shape:

```text
LanguageProviderCapabilities
  language
  provider
  provider_version
  routes: Vec<RouteExecutionCapability>

RouteExecutionCapability
  route
  scope
  supported
  execution_modes
  preferred_execution_mode
  max_safe_parallelism
  supports_cancellation
  requires_shared_workspace_load
  limitations
```

Suggested execution modes:

```text
serial
parallel_shared_analysis
parallel_snapshot_clone
single_owner_worker
independent_load_pool
unsupported
```

The runtime should combine:

- provider route capabilities;
- `.refactor-radar/config.toml` extractor settings;
- CLI overrides;
- available machine parallelism;
- DB writer backpressure settings;
- route freshness requirements.

The selected plan should be recorded with the route run:

```text
requested_execution_mode
actual_execution_mode
requested_jobs
actual_jobs
fallback_reason
provider_limitations
```

For Rust, this handshake will be backed by the rust-analyzer-lib source spike.
For C#, the same handshake should let the provider report whether the
language-server path supports concurrent requests, cancellation, incoming
calls, outgoing calls, or request serialization. This keeps C# from becoming a
Rust-shaped special case when it is implemented.

## Pipeline Shape

Use phases.

Phase 1: workspace load.

```text
discover files
load rust-analyzer analysis once
build file and line-index data
```

Phase 2: document symbols.

```text
extract document symbols for source files
persist document-symbol graph through write manager
build current symbol index
```

Phase 3: relation extraction.

```text
enqueue eligible reference targets
enqueue eligible callable symbols
run reference worker pool
run call worker pool
group outputs deterministically
send typed batches to writer
```

Phase 4: route completion.

```text
wait for all reference jobs
complete or fail rust.references route
close stale reference edges only on success
wait for all call jobs
complete or fail rust.calls route
close stale call edges only on success
finish runs
```

References and calls can run concurrently after document symbols are persisted
or after both routes share a trusted in-memory current symbol index.

## Worker Model

Use bounded queues.

Suggested job types:

```text
ReferenceJob
  target_node_id
  target_file_path
  target_selection_range
  target_name

CallJob
  caller_node_id
  caller_file_path
  caller_selection_range
  caller_name
```

Suggested outputs:

```text
ReferenceJobResult
  target_node_id
  extracted_references
  skipped_external
  skipped_unresolved
  diagnostics

CallJobResult
  caller_node_id
  extracted_calls
  skipped_external_targets
  skipped_unresolved_targets
  diagnostics
```

The worker pools should not write to SQLite directly. They should send typed
write batches or route-local aggregation results to the write manager.

## Worker Counts

Make worker counts configurable.

Suggested CLI flags:

```text
--jobs <n>
--reference-jobs <n>
--call-jobs <n>
--serial
```

Default policy:

```text
jobs = min(physical_cores - 1, 8)
reference_jobs = split from jobs
call_jobs = split from jobs
```

Keep the first default conservative. Too many rust-analyzer queries can produce
cache contention, memory pressure, or worse wall-clock time.

## Config Settings

VS-Threaded may extend `.refactor-radar/config.toml` with extractor threading
settings:

```toml
[extractor]
mode = "threaded"
jobs = 16
reference_jobs = 10
call_jobs = 6
```

Rules:

- `[database].path` remains owned by VS-Config;
- `--serial`, `--jobs`, `--reference-jobs`, and `--call-jobs` override
  `[extractor]` values;
- `reference_jobs + call_jobs` must not exceed `jobs` unless the user
  explicitly overrides both route-specific values;
- invalid worker counts should fail before extraction starts;
- `mode = "serial"` must remain available for parity checks and debugging.

## Determinism

Threaded extraction must not create nondeterministic graph churn.

Rules:

- sort input jobs by stable symbol key;
- sort worker outputs before grouping when route results are merged;
- use stable edge IDs and grouping keys exactly as the serial route does;
- write evidence in stable order when practical;
- keep summary counts independent of completion order;
- compare serial and threaded outputs in tests.

If exact evidence row insertion order differs but canonical graph facts match,
document that as acceptable only if no user-facing output depends on row order.

## Error And Cancellation Policy

One failed job fails its route unless the error is explicitly classified as a
skippable provider diagnostic.

Rules:

- route failure stops new jobs for that route;
- workers finish or cancel current jobs cooperatively;
- writer records route failure;
- failed routes do not close stale rows;
- partial writes from a failed route must not be marked complete;
- `rust-workspace-all` should return a nonzero exit code when a required route
  fails.

Skips are not failures when they match existing policy:

- external reference/call targets;
- unresolved call targets;
- symbols outside the tracked workspace.

Those should be counted in diagnostics and summaries.

## Progress Reporting

Threaded extraction should expose honest progress.

Recommended counters:

```text
files_discovered
document_symbol_files_done / total
reference_jobs_done / total
call_jobs_done / total
batches_queued
batches_committed
queue_depth
routes_complete
routes_failed
stale_edges_closed
```

Do not fake a precise time percentage. Report work-item progress and phase
state.

## CLI Work

Extend `rust-workspace-all` first.

Suggested examples:

```sh
cargo run -p semantic-graph-extract -- rust-workspace-all \
  --workspace-root . \
  --jobs 6
```

```sh
cargo run -p semantic-graph-extract -- rust-workspace-all \
  --workspace-root . \
  --serial
```

Keep `--serial` until threaded parity is proven.

## Tests

Facade/threading tests:

- shared loaded analysis can run multiple reference jobs concurrently;
- shared loaded analysis can run multiple call jobs concurrently;
- mixed reference and call jobs do not panic or corrupt state;
- worker count of one matches serial behavior;
- provider capabilities describe supported execution modes per route;
- runtime selection chooses the provider preferred mode when config allows it;
- runtime selection falls back cleanly when the requested mode is unsupported;
- selected execution mode and fallback reason are recorded in route metadata.

Pipeline tests:

- threaded references match serial references for the smoke workspace;
- threaded calls match serial calls for the smoke workspace;
- threaded `rust-workspace-all` matches serial current graph counts;
- route failure prevents stale closing;
- cancellation fails the route cleanly;
- progress events count all jobs once;
- deterministic output is stable across repeated threaded runs.

Performance tests:

- capture baseline serial wall-clock time;
- capture threaded wall-clock time;
- report speedup without making strict timing assertions in normal tests;
- log worker counts and batch sizes for smoke runs.

## Validation Commands

Expected validation after implementation:

```sh
SQLX_OFFLINE=true cargo test -p rust-analyzer-lib
SQLX_OFFLINE=true cargo test -p semantic-graph-extract
SQLX_OFFLINE=true cargo test -p semantic-graph-smoke-tests
SQLX_OFFLINE=true cargo run -p semantic-graph-smoke-tests
cargo run -p semantic-graph-extract -- rust-workspace-all \
  --workspace-root . \
  --serial
cargo run -p semantic-graph-extract -- rust-workspace-all \
  --db .local/rust-workspace-extract-threaded.db \
  --workspace-root . \
  --jobs 6
```

Add a comparison command or test helper that verifies the two DBs have matching
current canonical facts for files, nodes, edges, occurrences, evidence counts,
and route statuses.

Do not present the work as complete while `cargo check`, tests, or smoke runs
emit warnings.

## Acceptance Criteria

VS-Threaded is complete when all of the following are true:

- provider route capabilities expose supported execution modes;
- the runtime selects an execution plan through the provider handshake;
- document symbols are refreshed before threaded relation routes start;
- references can run on a bounded worker pool;
- calls can run on a bounded worker pool;
- reference and call extraction can overlap after document symbols are current;
- workers do not perform direct SQLite writes;
- route success/failure and stale closing are correct;
- threaded output matches serial output for current canonical graph facts;
- selected execution mode is recorded for each relation route run;
- progress reporting exposes phase and job counters;
- worker counts are configurable;
- worker counts can be loaded from `.refactor-radar/config.toml`;
- validation commands pass without warnings.

## Risks And Decisions To Preserve

- Shared rust-analyzer analysis safety must be proven, not assumed.
- More threads can be slower if they thrash caches or memory.
- Deterministic graph output matters more than maximum throughput.
- Route completion is a barrier. Do not stale-close while workers are still
  producing facts.
- Keep the installed `rust-analyzer` CLI out of the design.
