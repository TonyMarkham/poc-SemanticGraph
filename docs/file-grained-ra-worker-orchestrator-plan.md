# File-Grained RA Worker Orchestrator Plan

This plan captures the next preferred simplification after benchmarking the
current route-worker plus RA-pool design.

## Problem Statement

The current threaded extractor shape is correct for streaming writes, but the
benchmarks show it is not faster than serial on this workload.

Observed behavior:

- Early in the run, all cores are saturated.
- Later in the run, CPU usage becomes uneven.
- Adding multiple persistent RA workers did not materially improve route time.
- Split RA pools increased memory use; split 8/8 peaked around 38 GB RSS.

Likely causes to test next:

- Fixed per-route workers create uneven long-tail scheduling.
- Per-target work items are too small and noisy.
- Mapping one target result at a time may cost more than the serial batch path.
- Reference and call lanes are over-separated for the current workload.

## Proposed Shape

Use file-grained semantic work items.

The extractor still performs document discovery and document-symbol extraction
first. Then it groups all relation targets declared in each file:

```text
FileSemanticWork
  file_path
  reference_targets declared in file
  call_targets declared in file
```

Important: a reference target declared in file A can produce references across
the whole workspace. The file is the scheduling unit, not the search boundary.

Each warm RA worker owns one persistent `LoadedAnalysis` and processes one file
work item at a time:

```text
for reference target in file_work.reference_targets:
  query references_for_symbol

for call target in file_work.call_targets:
  query outgoing_calls_for_symbol
```

The worker returns:

```text
FileSemanticResult
  file_path
  reference_sets
  call_sets
```

The orchestrator receives file results, maps them with global document-symbol
context, emits DB writes immediately, and awaits DB-manager success before
counting that file result complete.

## Execution Flow

```text
discover workspace Rust source files
start RA worker pool
extract document symbols
persist document symbols
derive reference and call targets
group targets by declaring file
start reference/call route statuses
enqueue file work items

while file work remains:
  assign next file work item to first available RA worker
  receive FileSemanticResult
  map reference sets
  persist reference writes immediately
  map call sets
  persist call writes immediately
  update route summaries

after all file work succeeds:
  complete reference route
  close stale reference edges
  complete call route
  close stale call edges
  shutdown RA pool
  shutdown DB manager
```

If a file work item fails:

- mark the affected route failed
- do not stale-close that route
- finish the run as failed
- shut down workers cleanly

## Why This Is Simpler

This removes the current two-layer scheduling shape:

```text
route workers -> RA worker pool -> result mapping/writes
```

and replaces it with:

```text
file work queue -> RA worker pool -> result mapping/writes
```

Expected benefits:

- Dynamic scheduling evens out the long tail better than fixed route loops.
- Each worker receives meaningful batches instead of one target at a time.
- Reference and call work for the same file may benefit from locality.
- The extractor has one orchestration model instead of separate reference/call
  lane schedulers.
- The DB write contract remains streaming and acknowledged.

## Types To Add

Keep one primary type per file.

Suggested `rust-analyzer-lib` types:

```text
semantic/file_semantic_work.rs
semantic/file_semantic_result.rs
semantic/file_semantic_worker_pool.rs
```

Suggested extractor orchestration types:

```text
workspace_all/file_relation_work.rs
workspace_all/file_relation_result.rs
workspace_all/file_relation_orchestrator.rs
```

Exact names can change to fit local module patterns, but keep modules small.

## Config

Prefer one main RA pool knob for this path:

```toml
[extractor]
mode = "threaded"
jobs = 30
analysis_workers = 8
```

Possible interpretation:

- `jobs` remains relation orchestration pressure if still useful.
- `analysis_workers` controls the number of warm RA workers.
- Omit `reference_jobs`, `call_jobs`, `reference_analysis_workers`, and
  `call_analysis_workers` from normal config unless explicitly benchmarking old
  route-lane behavior.

If file-grained orchestration replaces route workers entirely, consider
renaming the runtime knob later to avoid ambiguity:

```toml
[extractor]
mode = "threaded"
analysis_workers = 8
```

## Benchmark Matrix

Use release builds and fresh DB paths.

Baseline comparisons:

```text
serial
current threaded 15/15, analysis_workers=1
current threaded 15/15, shared analysis_workers=8
file-grained analysis_workers=2
file-grained analysis_workers=4
file-grained analysis_workers=8
file-grained analysis_workers=16 if memory permits
```

Collect:

- total wall time
- document-symbol query/map/persist time
- file work items total
- per-file RA query time min/p50/p95/max
- per-file map time min/p50/p95/max
- per-file persist time min/p50/p95/max
- worker idle time if practical
- peak RSS from system monitor
- edge and occurrence counts for parity

## Guardrails

- Do not cache all DB writes in the orchestrator for one final bulk dump.
- Continue emitting writes as semantic facts are mapped.
- Await DB-manager success before counting a file result complete.
- Keep batch writes as a future DB-manager feature, not extractor-owned state.
- Preserve typed errors and `error-location`; do not add `anyhow`.
- Do not use `super` imports or glob imports.
- Keep one Rust type per module file.
- Treat submodules as read-only.

## Open Questions

- Should workers return one `FileSemanticResult` at the end of a file, or stream
  partial reference/call results back while processing a large file?
- Should very large files be split into chunks by target count?
- Is the current per-target mapping path the real bottleneck, and should mapping
  be changed to support batches of returned sets per file?
- Should route status use one shared run for both routes or keep separate
  reference and call route runs as currently implemented?

## Implementation Result

The file-grained path was implemented with:

- `rust-analyzer-lib::FileSemanticWork`
- `rust-analyzer-lib::FileSemanticResult`
- `AnalysisWorkerHandle::file_semantic_work`
- one shared warm `AnalysisWorkerPool`
- one file work queue consumed by dedicated RA worker handles

The checked-in config was simplified to:

```toml
[extractor]
mode = "threaded"
analysis_workers = 8
```

Release benchmark on this workspace:

| Mode | Total ms | File relation ms | File work items | Analysis workers |
| --- | ---: | ---: | ---: | ---: |
| file-grained | 19,513 | 15,885 | 222 | 8 |

This beats the previous measured serial baseline of about 26.3s and the
previous threaded route-worker shape of about 42-46s.
