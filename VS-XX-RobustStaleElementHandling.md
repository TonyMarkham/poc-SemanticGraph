# VS-RobustStaleElementHandling

Status: Draft implementation plan
Date: 2026-06-15

## Goal

Define the stale element handling workflow for semantic graph extraction,
independent of how extraction is triggered.

The intended end state is:

- deleted code facts are marked stale, not deleted;
- current graph reads exclude stale nodes and edges by default;
- historical evidence and occurrences remain available for provenance;
- failed or partial extraction routes never stale-close old facts;
- file, crate, package, and workspace extraction all use the same route-scoped
  stale policy;
- connected edges are handled deliberately when endpoint nodes go stale.

## Non-Goals

- Do not add file watcher behavior here.
- Do not make stale handling depend on a file watcher.
- Do not physically delete canonical graph rows.
- Do not rewrite historical evidence, occurrences, route observations, or old
  extraction runs.
- Do not infer semantic edge deletion from a route that did not actually check
  that edge kind.

## Current Storage Semantics

Canonical graph rows live in `nodes` and `edges`.

Current rows have:

```sql
valid_to_run_id IS NULL
```

Stale rows have:

```sql
valid_to_run_id IS NOT NULL
```

Evidence tables are historical:

- `occurrences`
- `edge_evidence`
- `route_observations`
- `extraction_runs`
- `extraction_route_status`

Those tables are not stale-closed. They remain tied to the run that produced
them.

## Core Rule

A route may stale-close only the facts it is authoritative for, and only after
that exact route scope completed successfully.

The stale key is:

```text
workspace_id
route
scope
scope_key
provider
run_id
```

If the route fails, is cancelled, times out, or writes only partial
observations, it must not stale-close old facts.

## Route Authority

Document-symbol routes are authoritative for:

- nodes defined in the extracted scope;
- `contains` edges produced from the extracted symbol tree;
- definition/declaration occurrences for the extracted scope.

Reference routes are authoritative for:

- `references` edges for the extracted reference scope;
- reference occurrences produced by that route.

Call routes are authoritative for:

- `calls` edges for the extracted call scope;
- call occurrences produced by that route.

Occurrences and evidence remain historical even when their canonical node or
edge becomes stale.

## Workflow

Every extraction route must follow this shape:

```text
1. start extraction run
2. start route status for route/scope/scope_key/provider
3. extract current facts for that route scope
4. upsert current canonical files/nodes/edges
5. insert current occurrences and evidence
6. record route observations for every canonical node/edge this route saw
7. complete route status
8. close stale canonical rows for that same route/scope/scope_key/provider
9. finish extraction run
```

Step 8 is valid only after step 7 succeeds.

If any step before route completion fails:

```text
mark route failed
mark run failed when appropriate
do not close stale rows
```

## Stale Close Algorithm

For a completed route run, stale closure compares previous observations for the
same route key against current observations for the current run.

For nodes:

```sql
UPDATE nodes
SET valid_to_run_id = current_run_id
WHERE valid_to_run_id IS NULL
  AND id was observed by an older run for this route key
  AND id was not observed by current_run_id for this route key
```

For edges:

```sql
UPDATE edges
SET valid_to_run_id = current_run_id
WHERE valid_to_run_id IS NULL
  AND id was observed by an older run for this route key
  AND id was not observed by current_run_id for this route key
```

Upserts for reobserved nodes and edges must set:

```sql
valid_to_run_id = NULL
```

That gives the extractor limited self-healing: a later successful run can
reopen a canonical row that was stale-closed by an earlier run.

## Connected Edge Policy

When a node becomes stale, current graph queries must not show active edges
attached to that stale node.

The minimum required rule is read-side filtering:

```sql
edge.valid_to_run_id IS NULL
AND src_node.valid_to_run_id IS NULL
AND dst_node.valid_to_run_id IS NULL
```

That prevents stale nodes from leaking into the current graph even if an
attached semantic edge has not yet been revalidated by its own route.

Write-side connected-edge handling should be explicit:

- `contains` edges are closed by the document-symbol route that owns them.
- `calls` and `references` edges are closed by their own routes when those
  routes complete.
- If an endpoint node is stale, incident `calls` and `references` edges should
  be treated as affected work, not blindly closed unless the selected policy
  says endpoint staleness invalidates them immediately.

Recommended first policy:

```text
1. close route-owned edges normally;
2. filter incident edges out of current reads when either endpoint is stale;
3. enqueue or schedule affected reference/call refreshes for stale node ids;
4. close those semantic edges only when the relevant semantic route completes.
```

This avoids claiming a semantic edge is gone before the route that owns that
edge has actually checked it.

## Read Policy

Read APIs must be explicit about stale visibility.

Current graph reads must filter:

```sql
nodes.valid_to_run_id IS NULL
edges.valid_to_run_id IS NULL
edge source node valid_to_run_id IS NULL
edge destination node valid_to_run_id IS NULL
```

Historical reads may include stale rows, but their API should say so.

Recommended read modes:

```text
CurrentOnly
IncludeStale
StaleOnly
```

The visualizer default should be `CurrentOnly`.

## DB Manager Responsibilities

The DB manager must own all stale writes.

Required commands:

- start route status;
- complete route status;
- fail route status;
- record route observations;
- close stale nodes for a completed route;
- close stale edges for a completed route;
- optionally collect and return stale node/edge ids from close operations;
- optionally mark or report affected semantic work for incident edges.

No extractor worker should write stale state directly.

## Extractor Responsibilities

The extractor must:

- choose the correct route, scope, and scope key;
- record observations for every canonical fact the route is authoritative for;
- call route completion only after all current facts and observations are
  written;
- skip stale closure when extraction is incomplete;
- use the same workflow for single-file, crate, package, and workspace routes.

The extractor must not:

- stale-close rows for routes it did not run;
- stale-close semantic edges only because a file changed;
- hide stale rows by deleting them.

## Implementation Tasks

1. Audit all current read queries and add explicit current/stale filtering.
2. Add tests proving current graph queries exclude edges whose endpoint nodes
   are stale.
3. Add tests for single-file document-symbol stale closure:
   - deleted symbol becomes stale;
   - deleted `contains` edge becomes stale;
   - reintroduced symbol reopens the canonical node.
4. Add tests proving failed routes do not stale-close old facts.
5. Add tests proving reference and call routes close only their own observed
   edge kinds.
6. Add DB manager support to return stale node/edge ids from close operations
   if affected-work scheduling needs them.
7. Add an affected-edge policy implementation after the read filtering audit is
   complete.

## Acceptance Criteria

- Re-running extraction without source changes produces zero stale closures.
- Deleting a symbol and rerunning the owning document-symbol route marks that
  symbol stale.
- Reintroducing the symbol and rerunning the route reopens the same canonical
  node id.
- Current graph queries do not return stale nodes.
- Current graph queries do not return edges connected to stale nodes.
- Failed routes preserve previous current graph state.
- Historical evidence remains queryable after canonical rows become stale.
