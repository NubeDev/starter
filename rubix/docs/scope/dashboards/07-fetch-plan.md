# 07 — Batched historical fetch (FetchPlan) — deferred to v2

> **Tier:** scope (plan). Lifetime: weeks. Not referenced from code.
> See [README.md](./README.md). This file is the **deferred** piece
> — documented now so the IR doesn't drift before the work lands.

## What this file decides

The contract for `POST /api/v1/ui/series` and its `FetchPlan` /
`FetchItem` / `FetchPayload` shapes. Lifted from
[`examples/rubix-agent/crates/dashboard-transport/src/fetch_plan.rs`](../../../../examples/rubix-agent/crates/dashboard-transport/src/fetch_plan.rs).

**Not built in v1.** v1 charts get their data via the
subscription plan (live slot updates) plus a small in-memory
buffer in the renderer. Historical pulls (e.g. "last 7 days of
disk usage at 5-minute buckets") need `FetchPlan`.

## Why defer

Three reasons:

1. v1 dashboards reproduce today's hand-coded views, all of
   which run on **live** values; no chart currently renders a 7-day
   history without ClickHouse plumbing already in place via
   `starter-warehouse` for specific tools (the existing
   `system_disk_history` table).
2. `FetchPlan` cuts deeply into `domain-rsql-aggregation`,
   `data-tsdb`, and the `TelemetryRepo` trait — work that should
   ride alongside a broader warehouse cleanup.
3. Locking the wire shape early lets the IR carry chart variants
   that *intend* `FetchPlan` semantics
   (`ChartSource::Series`, `SeriesByKind`, `SeriesFromRsql`) but
   degrade gracefully in v1.

## v1 degradation rule (already shipping)

The IR's `ChartSource` enum has variants for `series`, `rows`,
`series_by_kind`, `series_from_rsql`, and `static`. In v1:

| Variant | v1 behaviour |
|---|---|
| `static` | Inline data — works. |
| `series` (single slot, single entity) | Subscribed via `SubscriptionPlan`; the renderer keeps a ring buffer of the last `N` updates and graphs that. Cheap, no historical depth. |
| `series_by_kind` | Same as `series` but multiple entities; one ring buffer each. |
| `rows` | `QueryEngine` returns rows once on resolve; no live update in v1. |
| `series_from_rsql` | Renderer surfaces a `Diagnostic` "historical RSQL series requires FetchPlan (v2)". Page still renders, chart shows the diagnostic. |

The `Diagnostic` route means a page authored with future
intent **doesn't crash today** — the unsupported widget shows a
clear message instead.

## v2 wire shape

```rust
pub struct FetchPlan { pub items: Vec<FetchItem> }

pub enum FetchItem {
    Telemetry { widget_id, node_id, slot, field?, range, bucket_ms, agg? },
    Rows      { widget_id, spec: AggregationSpec },
    SeriesFromRsql { widget_id, spec, range, bucket_ms },
    Slot      { widget_id, node_id, slot, field? },
    #[serde(other)] Unknown,
}

pub enum FetchPayload {
    Telemetry { points: Vec<TelemetryPoint> },
    Rows      { result: AggregationResult },
    Slot      { value: JsonValue },
    Error     { what: WhatTag, message: String },  // partial-success
}

pub struct FetchResponse { pub items: BTreeMap<String, FetchPayload> }
```

**Partial-success contract.** A 1-of-12 failure does **not**
500 the request; the failing item maps to `FetchPayload::Error`
with a stable `what:` tag. HTTP 500 is reserved for plan-decode
and auth failures.

## v2 layering

`crates/starter-sdui-routes/src/routes/series.rs` (new) holds the
HTTP handler. The dispatcher
`crates/starter-sdui-fetch-plan/src/lib.rs` (new crate, opt-in)
implements `execute_fetch_plan`. The split mirrors today's
`starter-sdui-routes` vs `starter-ui-bindings` separation: routes
crate owns the HTTP, dispatcher crate owns the policy.

Rubix-side: `rubix-agent/src/sdui/fetch_plan.rs` (new) wires the
dispatcher to rubix's `TelemetryRepo` (ClickHouse) and to the
`QueryEngine` impl.

## Limits (R8 — same shape as resolve)

- `items.len() ≤ 64`.
- Total wall-clock deadline 5 s; per-item 2 s.
- Serialised response cap 4 MiB.
- Tag every overflow with a stable `what:` tag — `series_too_many_items`,
  `series_total_deadline`, `series_response_too_large`.

## When v2 lands

Triggered by either of:

- The first dashboard that needs > 1 hour of historical context
  for a chart that isn't `system_disk_history`.
- The AI builder asking for "show me errors per day for the
  last month" — `series_from_rsql` is the natural fit.

Until then, this file holds the contract so authored pages
written today degrade predictably tomorrow.

## Acceptance for v1 (degradation, not the feature)

1. A chart page authored with `ChartSource::SeriesFromRsql`
   renders with a `Diagnostic` widget in v1 and does not crash.
2. The IR's `ChartSource` discriminated enum remains stable
   between v1 and v2 (no rename, only additional variants if
   needed).
3. The v2 work has a `docs/scope/dashboards/07-fetch-plan.md`
   that survives long enough to be promoted to `docs/design/sdui/`
   when the crate ships.
