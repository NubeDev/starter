# Feature: Flow Workbench — Stream · Table · Transform in One Panel

> Status: **DESIGN + phased build.** Phase 1 (Table) in progress.
> Motivation (user, 2026-06-10): debugging a flow means hopping between the
> Debug drawer (stream), the Explore page (query the sink table, retyping its
> name), and the flow editor (transforms). The flow already knows its sink table,
> connection, and pipeline — the tooling should too. One flow-scoped panel.

## The idea

Grow the existing Debug drawer into a **Workbench** with three tabs over one
shared context (the flow you opened it on):

```
┌ Workbench · zenoh-typed-ingest ───────────────────────────── (live) ─ ✕ ┐
│  [ Stream ]   [ Table ]   [ Transform ]                                  │
│                                                                          │
│  ── debug canvas: zenoh ─▶ json_to_arrow ─▶ postgres (live counts) ──    │
│                                                                          │
│  Stream:    per-node counters + sampled rows + run logs   (built)        │
│  Table:     query the flow's SINK table, pre-wired         (phase 1)     │
│  Transform: edit pipeline → dry-run → input/output diff    (phase 3)     │
└──────────────────────────────────────────────────────────────────────────┘
```

The three tabs answer the three questions you actually ask while debugging:

| Tab | Question | Data source |
|-----|----------|-------------|
| **Stream** | "Is data flowing, and what does it look like at each node?" | Debug SSE (`/flows/{id}/debug/stream`) — already built |
| **Table** | "What actually landed in the table?" | the flow's sink table, queried read-only |
| **Transform** | "What does my pipeline *do* to a row — without writing anything?" | `/flows/{id}/dry-run` (already exists) |

The win: **no page-switch, no retyping the table name, no guessing which
datasource**. The flow is the single context; the panel derives everything from
it.

---

## Why this is mostly wiring, not new machinery

- **Stream** — done (the debug tap + SSE + `useFlowDebug`).
- **Transform** — `POST /api/v1/flows/dry-run` already runs `input + pipeline`
  against a bounded collector sink, no persistence, and returns
  `{columns, rows, stats, error}`. The Transform tab is a pipeline editor bound
  to this endpoint. Its result shape == a query result, so it shares the renderer.
- **Table** — the one genuinely new endpoint (see below), but it reuses the
  existing read-only query path (`run_query`: `SET TRANSACTION READ ONLY` + bound
  params + caps).

---

## The Table query path — `POST /api/v1/flows/{id}/table/query` (decided)

**Why a flow-scoped endpoint, not "auto-match a datasource":** the flow *owns*
its sink. Matching a registered datasource by connection URI is fragile (none may
exist; `localhost` vs `127.0.0.1`; different db user), and making the user create
a datasource just to peek at their own flow's output is the exact friction we're
removing. A flow-scoped endpoint is the better UX *and* the cleaner model.

Shape:

```jsonc
// POST /api/v1/flows/{id}/table/query
{ "sql": "SELECT * FROM {table} ORDER BY \"timestamp\" DESC LIMIT 50",  // optional; default = last-N
  "variables": [], "time_range": null }                                 // same as /query
// → { columns, rows, stats }   (identical to /api/v1/query)
```

Semantics:
- Reads the flow's `output` config; supports `{type:postgres, uri, table}` (and
  later the `datasource` sink by resolving its id). Opens a read-only pool on the
  sink `uri` (server-side secret, never from the request).
- Runs through `run_query` — `SET TRANSACTION READ ONLY` is the security boundary
  (any write/DDL is rejected by PG itself), bound params, statement timeout, row
  cap. No new guard logic.
- `{table}` is exposed as a convenience macro so the default query and the editor
  placeholder don't hardcode the name; the user can also just type it.
- Tenant-gated like every flow route (RLS + principal).

Non-postgres sinks (`sse`, `broadcast_store`, `drop`) have no table → the Table
tab is hidden/disabled for those flows.

---

## Phased build

### Phase 1 — Table tab (highest daily-pain relief) ✅ DONE (2026-06-10)
- [x] Backend: `POST /flows/{id}/table/query` — reads the flow output, opens a
      read-only pool on the sink connection (raw `postgres` uri, or a resolved
      `datasource` sink's conn), runs through `run_query`. `{table}` macro,
      `limit` clamp (≤500). DTO `FlowTableQueryRequest` → openapi → TS codegen.
      `table_query.rs`, 6 unit tests.
- [x] Default query `SELECT * FROM {table} ORDER BY 1 DESC LIMIT 50`.
- [x] UI: Table tab in `DebugDrawer.tsx` — SQL textarea prefilled, Run button,
      shared `ResultTable`. Hidden when `sinkTableName(output)` is null
      (sse/drop/broadcast sinks). `api/flows/table.ts` + `FlowTableQueryRequest`
      type. Typechecks + builds.
- [x] Verified e2e: default preview returned rows; `{table}` aggregate
      (`elec 740 / water 739`); **read-only guard rejects `DROP TABLE`**, table
      intact. No datasource setup, no retyping the table name.

### Phase 2 — Unified node-click pivot ✅ DONE (2026-06-10)
- [x] Tabs are controlled (`tab` state). Clicking a node on the debug canvas
      (`onNodeClick`) selects it and pivots the bottom panel: the **sink**
      (output) node → the Table tab; any other node → its live Values. One
      gesture to pivot stream ↔ table. (`DebugDrawer.tsx`.) Per-node table row
      click still just selects (you're already reading that table). Typechecks.

### Phase 3 — Transform tab (dry-run sandbox) ✅ DONE (2026-06-10)
- [x] UI: `TransformTab` — editable pipeline JSON (left) bound to the existing
      `/flows/dry-run` via `useDryRun()`; transformed output (right) via the
      existing `DryRunResult`. Uses the flow's real `input`; runs against a
      bounded collector, **no DB write, running flow untouched**. Parse-error
      gating on the JSON.
- [x] "Apply to flow" — explicit, separate button via `useUpdateFlow()`; only
      enabled when the JSON is valid and changed; notes that a restart is needed
      for the live run to pick it up. Never silent.
- [x] Verified live: dry-run of `zenoh → json_to_arrow` returned a 3-row typed
      sample (`error: None`), and the sink table did **not** grow from the
      dry-run (collector sink, not postgres). Typechecks + builds.

---

## Result — the workbench is complete

From a running flow's Debug drawer you now **never leave the panel** to:
- **Stream** — watch per-node counters + sampled rows + logs (canvas + tabs).
- **Table** — query the flow's own sink table (prefilled, read-only); reach it in
  one click on the sink node.
- **Transform** — edit the pipeline and dry-run input→output with no write, then
  Apply explicitly.

One flow context; streaming, the database, and transformations side by side.

---

## Acceptance criteria

- ✅ From a running flow, **without leaving the panel**, you can: watch the
  stream, query the sink table, and dry-run a pipeline change.
- ✅ Table tab needs no datasource setup and no retyping the table name.
- ✅ Table query is read-only (a write/DDL is rejected) and tenant-scoped.
- ✅ Transform dry-run never writes to the DB or perturbs the running flow.
- ✅ The three views agree: sink-node stream counts ≈ table row growth; dry-run
  output columns == what the live processor emits.

---

## Known issues / fixes

- _none yet (design)_
