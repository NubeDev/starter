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

### Phase 2 — Unified node-click pivot
- [ ] Click a node on the debug canvas → bottom panel focuses that node's live
      samples; clicking the **sink** node also surfaces the Table query inline.
- [ ] One gesture to pivot stream ↔ table on the same node.

### Phase 3 — Transform tab (dry-run sandbox)
- [ ] UI: pipeline editor (the flow's `pipeline`, editable) bound to
      `/flows/{id}/dry-run`; show **input sample → transformed output** side by
      side, no DB write, running flow untouched.
- [ ] "Apply to flow" promotes the edited pipeline via the flow update endpoint
      (explicit, never silent).
- [ ] Test: add a `sql` processor step, dry-run, see the output columns change.

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
