# VS-FixCsharpThreads

## Purpose

Document the C# worker-pool threading finding from VS-10 follow-up benchmarking
and define the fix direction.

## Benchmark Observation

Command shape tested against the repo root Blazor solution:

```sh
./target/release/semantic-graph-extract csharp-solution \
  --db /tmp/csharp-root-slnx-full.db \
  --solution SemanticGraph.Visualizer.slnx
```

Result with the default one C# process worker:

```text
scope=solution mode=all files=27 nodes=168 contains_edges=141 references_edges=972 reference_occurrences=1130 calls_edges=130 call_occurrences=186 routes_complete=29
bench.actual_process_workers=1
bench.document_files=27
bench.files_discovered=27
bench.mode=csharp-process-pool
bench.process_workers=1
bench.routes=all
bench.total_ms=15750
```

Command shape with the CLI worker-count override:

```sh
./target/release/semantic-graph-extract csharp-solution \
  --db /tmp/csharp-root-slnx-full-8workers.db \
  --solution SemanticGraph.Visualizer.slnx \
  --process-workers 8
```

Result with eight C# process workers:

```text
scope=solution mode=all files=27 nodes=168 contains_edges=141 references_edges=972 reference_occurrences=1130 calls_edges=130 call_occurrences=186 routes_complete=29
bench.actual_process_workers=8
bench.document_files=27
bench.files_discovered=27
bench.mode=csharp-process-pool
bench.process_workers=8
bench.routes=all
bench.total_ms=76417
```

The graph counts matched, but the eight-worker run was much slower. That result
is not valid evidence that C# semantic extraction cannot benefit from
parallelism, because the C# worker pool is not actually parallel yet.

## Finding

`CSharpLsWorkerPool` currently pays the worker startup cost serially:

```rust
for _ in 0..worker_count {
    workers.push(CSharpLsWorker::start(...).await?);
}
```

Source:
`crates/csharp-ls-lib/src/semantic/csharp_ls_worker_pool.rs`

It also processes file semantic work serially even when multiple workers exist:

```rust
for (index, work) in work_items.into_iter().enumerate() {
    let worker_index = index % self.workers.len();
    results.push(self.workers[worker_index].file_semantic_work(work).await?);
}
```

So `--process-workers 8` currently means:

```text
start csharp-ls worker 1, wait
start csharp-ls worker 2, wait
...
start csharp-ls worker 8, wait
then run relation work in a mostly serial loop
```

The observed 76 second runtime is mostly a measurement of this footgun:
duplicated cold `csharp-ls` / Roslyn / MSBuild startup cost paid serially, plus
serial relation dispatch.

## Rust Comparison

The Rust worker-pool implementation does not have the same startup footgun.

`AnalysisWorkerPool::start` launches all worker startup threads first and only
then joins them:

```rust
for _ in 0..worker_count {
    startup_handles.push(thread::spawn(move || {
        AnalysisWorker::start(worker_workspace_root)
    }));
}
```

Rust document-symbol extraction also splits work across workers and uses
`tokio::spawn` per assignment. Rust relation extraction starts one Tokio task
per worker and pulls file work from a shared queue.

Relevant files:

- `crates/rust-analyzer-lib/src/semantic/analysis_worker_pool.rs`
- `crates/semantic-graph-extract/src/workspace_extraction/threaded_workspace_extraction_runner.rs`

This matters because previous Rust benchmarking showed that eight parallel
`rust-analyzer-lib` workers were the best local performance point. The C#
worker-count surface copied the setting, but not the concurrency behavior.

## Correct Interpretation

Confirmed:

- C# extraction works correctly with one worker.
- C# `--process-workers 8` returns the same graph counts as one worker.
- The current C# eight-worker benchmark is misleading because worker startup
  and semantic work dispatch are serial.

Unknown:

- Whether multiple `csharp-ls` processes improve C# extraction after startup and
  dispatch are made truly concurrent.
- Whether Roslyn/MSBuild/NuGet/global package cache/build-host file access
  contention will limit scaling.
- Whether one warm `csharp-ls` process with in-process parallelism would beat
  multiple `csharp-ls` processes.

## Fix Direction

### 1. Make C# Worker Startup Concurrent

Use Tokio tasks or `JoinSet` to start all `CSharpLsWorker`s concurrently.

Target behavior:

```text
spawn worker-start future 1
spawn worker-start future 2
...
spawn worker-start future N
await all
```

Do not add a broad `futures` dependency unless it is already desired elsewhere.
`tokio::task::JoinSet` is sufficient.

### 2. Make C# File Semantic Work Dispatch Concurrent

Split `FileSemanticWork` items across workers and run one task per worker.

Each individual `CSharpLsWorker` should still process its own JSON-RPC requests
sequentially because it owns one stdio client. The concurrency boundary is
across workers, not within one worker, unless a proper multiplexed JSON-RPC
client is added later.

Preserve input order in the returned result vector if callers depend on it. If
ordering is not semantically required, document that clearly.

### 3. Add Phase Benchmarks Before Retesting

Add C#-specific timing labels so future results explain where time is spent:

```text
bench.worker_start_ms
bench.discovery_ms
bench.document_symbols_ms
bench.references_ms
bench.calls_ms
bench.persistence_ms
bench.shutdown_ms
bench.total_ms
```

If practical, also add per-worker startup timing:

```text
bench.worker_start.worker_0_ms
bench.worker_start.worker_1_ms
...
```

This separates:

- `csharp-ls` startup
- MSBuild/Roslyn solution load
- document-symbol query time
- reference query time
- incoming-call query time
- SQLite persistence time

### 4. Rerun Scaling Benchmarks

After the C# pool is actually concurrent, rerun:

```text
--process-workers 1
--process-workers 2
--process-workers 4
--process-workers 8
```

Use the same solution and fresh disposable DBs:

```sh
./target/release/semantic-graph-extract csharp-solution \
  --db /tmp/csharp-root-slnx-workers-N.db \
  --solution SemanticGraph.Visualizer.slnx \
  --process-workers N
```

Acceptance baseline:

- Graph counts must match the one-worker run.
- Worker startup should not be serially multiplied by worker count.
- If runtime still worsens with worker count, the benchmark should show whether
  the cost is startup contention, route work contention, or persistence.

## Later Optimization Track

Even with a fixed process worker pool, the best C# architecture may not be many
`csharp-ls` processes. Roslyn can share immutable `Compilation` / `SyntaxTree`
snapshots and per-tree `SemanticModel`s inside one process for read-only
analysis.

A stronger long-term design may be a fork or sidecar that exposes a custom
batch method inside one warm Roslyn process:

```text
semanticGraph/extractSolution
```

That method could:

- load the solution once;
- get project compilations once;
- parallelize over documents/syntax trees in-process;
- compute symbols, references, and incoming calls;
- return compact graph DTOs.

That would avoid both repeated process startup and high-volume LSP round trips.
Do not pursue this until the simple worker-pool concurrency fix and phase
benchmarks have been measured.

## Recommended Near-Term Default

Keep `[csharp].analysis_workers = 1` as the default until the worker pool is
fixed and re-benchmarked. With the current serial implementation, values above
one are misleading and usually slower.
