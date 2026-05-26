# Stage 05 — large dataset in the dashboard

## Scope

**In:** an SDUI dashboard `dashboard.data-flow-site-a` that renders
**30 days** of L2 data for all three meters without freezing the
browser. Built via `rubix.dashboard.page_set` (the AI-builder verb,
per [design/sdui/tools/README.md](../../design/sdui/tools/README.md)).

**Out:** dashboard polish (custom themes, deep-link state), drill-
down panels per anomaly diagnostic (stage 04 alerts can link out,
but the alert UI itself is separate), the scheduled
`rubix.analytics.report` path (separate session — see
[design/reports/README.md](../../design/reports/README.md)).

## Why this stage is its own thing

A naive dashboard SELECTs every L2 row in the window and ships
the JSON to the browser. For 30 days × 3 meters × 1-minute
buckets that's ~130k rows. The browser chokes; the chart library
spends 2 s on layout; the user thinks the stack is broken.

The fix is **server-side downsampling** before the JSON ever
leaves the agent. Stage 05 proves the dashboard verbs honour
that.

## L3 mart (locked)

```sql
CREATE TABLE rubix.meter_readings_15m (
  tenant_id    String,
  meter_id     String,
  kind         LowCardinality(String),
  unit         LowCardinality(String),
  bucket_start DateTime,                  -- floor(epoch_ms / 900_000) * 900
  value_avg    Nullable(Float64),
  value_min    Nullable(Float64),
  value_max    Nullable(Float64),
  quality_mix  Map(LowCardinality(String), UInt32)
) ENGINE = ReplacingMergeTree
PARTITION BY toYYYYMM(bucket_start)
ORDER BY (tenant_id, meter_id, bucket_start);
```

15-minute buckets is the dashboard's default zoom. 30 days × 3
meters × 96 buckets/day ≈ **8640 rows** — render-friendly. Use
L2 directly only when the user zooms in to ≤ 24 h.

Retention: **2 years** (L3 marts are narrow + cheap, per the
warehouse design doc's L1<L2<L3 rule).

## Pre-flight

- Stage 03 success bar green, and L2 has at least **24 hours** of
  data accumulated. (30 days is the target, but the dashboard
  must render correctly on the partial dataset too — that's the
  first success bar item.)
- The frontend dev server runs (`make frontend` from
  [rubix/](../../../) — see your terminal history).
- Auth cookies are valid for the dashboard verbs (the same
  `/tmp/smoke-cookies.txt` flow used in earlier stages).

## Steps

1. Land the L3 mart via `rubix.clickhouse.mart.create` (same
   shape as stage 03). Same data-loss caveat applies — see
   [design/clickhouse-rules/README.md §"mart.create undo data-loss caveat"](../../design/clickhouse-rules/README.md#martcreate-undo-data-loss-caveat).

2. Deploy a `com.rubix.data-flow.rollup` flow that materialises
   L2 → L3 every 5 minutes (same shape as the stage 03 cleaner;
   smaller SQL).

3. Build the dashboard body. The minimum component tree:

   ```
   page
   ├── row
   │   ├── kpi  "Site A — last 24h kWh"          (uses analytics.query)
   │   └── kpi  "Site A — last 24h L"            (uses analytics.query)
   └── row
       ├── chart line "elec.main 30d (15m buckets)"  (reads L3, value_avg)
       ├── chart line "elec.hvac 30d (15m buckets)"  (reads L3, value_avg)
       └── chart line "water.main 30d (15m buckets)" (reads L3, value_avg)
   ```

4. Write that tree via `rubix.dashboard.page_set` with
   `page_id = "dashboard.data-flow-site-a"` (the AI-builder
   verb that bypasses optimistic concurrency).

5. Add two new query templates to
   `rubix-tools/src/analytics/templates/` (`include_dir!` picks
   them up at compile time — see
   [design/reports/README.md §"rubix.analytics.query"](../../design/reports/README.md#rubixanalyticsquery)):

   - `meter_kwh_last_24h.sql`
   - `meter_litres_last_24h.sql`

   Each is one query against L3 with a `{tenant_id:String}` param.

6. Open the dashboard in the frontend, default 30-day window.

## Success bar

Stage 05 is done when **all four** are true:

1. The dashboard renders end-to-end without a browser warning
   ("page unresponsive", "long task > 500 ms") on **24 hours
   of L2 + 0 of L3** (the partial-data case).
2. After L3 has ≥ 7 days populated, the 30-day window renders in
   **< 1.5 s** wall-clock from `page_set` body fetch to chart
   first-paint. (Measure with browser devtools network +
   performance panes.)
3. The 30-day chart pulls **≤ 9000 rows total across all three
   meters** (proves the server downsampled to L3, did not stream
   L2). Verify by inspecting the `analytics.query` response size.
4. Zooming the chart to a 6-hour window switches to L2 and shows
   1-minute granularity (the cross-over works both ways).

Item 4 is gated by whichever chart node-kind in the SDUI tree
supports zoom-driven re-query — if that doesn't exist yet,
**drop item 4 and write a follow-up note** for it; do not block
this stage on building zoom-driven re-query.

## If it fails

In order, check:

1. **`page_set` rejects the body** — the ComponentTree is invalid.
   Run it through the validator first (see
   [design/sdui/tools/README.md §"per-verb sketch / page_set"](../../design/sdui/tools/README.md#page_setrs)).
2. **Chart renders but is empty** — the `analytics.query`
   template name is wrong, or the param shape doesn't match.
   Hit the verb directly via curl with the same params the
   dashboard sends.
3. **30-day window slow (> 5 s)** — the rollup flow isn't
   filling L3. Verify with
   `SELECT count() FROM rubix.meter_readings_15m`. If zero,
   stage 03's lookback window probably doesn't cover enough
   history for L3 to backfill — backfill once by hand.

Write follow-up notes as
`YYYY-MM-DD-data-flow-05-dashboard-<topic>.md` and stop.

## Decisions taken

- [ ] Built dashboard via `page_set` (AI builder)  /  [x] via bundled JSON seed (`dashboards_seed.rs` upserts `data-flow-site-a.json` on boot)
- L3 mart name: `rubix.meter_readings_15m`
- L3 retention: 730 days
- Rollup flow id: `com.rubix.data-flow.rollup`
- Dashboard `page_id`: `dashboard.data-flow-site-a`
