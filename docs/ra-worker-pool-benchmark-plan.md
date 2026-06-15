# RA Worker Pool Benchmark Plan

This checkpoint preserves the current profiling context and the intended next
implementation direction.

## Current Baseline

Release benchmarks were run with:

```sh
./target/release/semantic-graph-extract rust-workspace-all --workspace-root .
```

Observed totals on this machine:

| Mode | Total ms | Reference ms | Call ms |
| --- | ---: | ---: | ---: |
| serial | 26,254 | 10,294 extract | 9,247 extract |
| threaded 4/4 | 53,828 | 50,652 route | 23,243 route |
| threaded 8/8 | 45,207 | 42,113 route | 20,946 route |
| threaded 15/15 | 42,150 | 39,061 route | 21,633 route |

The current threaded extractor streams DB writes correctly, but it is not a
throughput win because semantic access is serialized through one
`single_owner_worker` rust-analyzer worker.

The machine has 32 cores and 64 GB RAM. During prior threaded benchmarks, memory
appeared to peak around 19.6 GB, leaving room to test duplicated persistent RA
analysis workers.

## Goal

Answer this profiling question:

```text
Does persistent parallel rust-analyzer analysis reduce reference/call route wall
time enough to beat the current serial baseline?
```

## Intended Architecture

Keep concurrency complexity inside `rust-analyzer-lib`.

Add an RA pool abstraction where each worker owns a persistent loaded analysis:

```text
AnalysisWorkerPool
  worker 0: LoadedAnalysis
  worker 1: LoadedAnalysis
  worker 2: LoadedAnalysis
  ...
```

The extractor should submit semantic jobs to the facade and continue focusing on
orchestration and persistence:

```text
extractor
  discover files
  request document symbols
  request references/calls
  stream mapped results to db-manager
```

The heavy context should stay resident in each RA worker:

- `LoadedAnalysis`
- provider version
- workspace/root identity
- VFS/path mapping held by rust-analyzer-lib internals

Per-job context should stay small:

- target
- route kind
- run/job id for diagnostics

## Pool Shapes To Test

Start with a generic `AnalysisWorkerPool` in `rust-analyzer-lib`, then benchmark
both shared and split lane shapes.

Shared pool:

```text
one pool of N workers receives document/reference/call requests
```

Split pools:

```text
reference lane
  N persistent RA workers
  receives ResolvedReferenceTarget jobs

call lane
  M persistent RA workers
  receives ResolvedCallTarget jobs
```

Split pools can be implemented by instantiating the same generic pool twice.

## Config Knobs

Prefer explicit names that describe the expensive resource:

```toml
[extractor]
mode = "threaded"
reference_jobs = 15
call_jobs = 15
analysis_workers = 1
reference_analysis_workers = 0
call_analysis_workers = 0
```

Interpretation:

- `analysis_workers` controls a shared RA worker pool.
- `reference_analysis_workers` and `call_analysis_workers` opt into split pools.
- If split pool values are nonzero, use split pools for those lanes.
- Keep defaults conservative.
- Allow aggressive values for this testbed.

## Benchmark Matrix

Use fresh DB files per run.

```text
serial baseline
single_owner_worker threaded 15/15
shared RA pool 2 workers, relation jobs 15/15
shared RA pool 4 workers, relation jobs 15/15
shared RA pool 8 workers, relation jobs 15/15
split RA pools 4 reference + 4 call, relation jobs 15/15
split RA pools 8 reference + 8 call, relation jobs 15/15
split RA pools 15 reference + 15 call, relation jobs 15/15 if memory permits
```

Compare:

- total wall time
- reference route time
- call route time
- RA worker startup time
- peak RSS
- DB write timing
- edge/occurrence counts for correctness parity

## Implementation Notes

- Keep one primary Rust type per module file.
- Do not use `super` imports or glob imports.
- Do not add `anyhow`; preserve typed errors and `error-location` style.
- Treat submodules as read-only.
- Preserve the DB-manager write acknowledgement behavior: extractor work should
  not finish a route until all emitted writes have returned success.
- The extractor should own shutdown after semantic discovery and DB writes are
  complete.
- Batch writes are still a future feature.
- Keep benchmark output as deterministic `bench.*` lines.

## Validation

For RA pool changes, run at minimum:

```sh
cargo fmt
cargo check
cargo clippy
cargo test -p rust-analyzer-lib
cargo test -p semantic-graph-extract
cargo test -p semantic-graph-config
cargo test -p semantic-graph-smoke-tests
cargo build --release -p semantic-graph-smoke-tests
./target/release/semantic-graph-smoke-tests
```

For benchmark runs, build release first:

```sh
cargo build --release -p semantic-graph-extract
```

Then run serial and threaded variants with fresh DB paths under `/tmp`.

## Pool Benchmark Result

After adding configurable `AnalysisWorkerPool`, release benchmarks on this
workspace produced:

| Mode | Total ms | Reference route ms | Call route ms | Notes |
| --- | ---: | ---: | ---: | --- |
| 15/15, shared analysis 1 | 43,913 | 40,811 | 22,500 | current single-owner equivalent |
| 15/15, shared analysis 4 | 44,585 | 41,403 | 22,860 | no throughput gain |
| 15/15, shared analysis 8 | 44,973 | 41,339 | 22,685 | no throughput gain |
| 15/15, split analysis 4/4 | 44,658 | 40,648 | 23,007 | no throughput gain |
| 15/15, split analysis 8/8 | 46,314 | 41,879 | 24,362 | peaked around 38 GB RSS |

The pool implementation is functional, but these measurements do not show a
route-time win from duplicating RA analyses on this workload. The next profiling
target should be the threaded route loop itself, especially per-target mapping
and persistence timing, because the threaded route maps one target result at a
time while the serial path maps the full batch in one pass.
