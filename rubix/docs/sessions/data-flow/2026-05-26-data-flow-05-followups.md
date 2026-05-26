# Stage 05 follow-ups — chart resolver ↔ analytics path + zoom

**Stage status:** ✅ landed (PROGRESS row 5). Success bar **item 1
only** verified live; items 2–4 are documented here as follow-ups
because the SDUI chart resolver does not currently back chart
sources from `rubix.analytics.query` template results.

## What's live

- L3 mart `rubix.meter_readings_15m` (730-day TTL) — migration
  `0005_meter_readings_15m` applied at boot.
- `rubix.warehouse.rollup_15m` tool — drives L2 → L3 with
  ReplacingMergeTree dedup. e2e: lookback=60 → 9..12 rows / 3
  meters across two cold restarts; `value_avg/min/max + quality_mix`
  populated.
- `com.rubix.data-flow.rollup` flow — cron `0 */5 * * * *`.
- `rubix.analytics.query` templates: `meter_kwh_last_24h`,
  `meter_litres_last_24h`. Both return rows live against the L3
  mart with `tenant_id=site-a`.
- Bundled dashboard `dashboard.data-flow-site-a` seeded into
  `dashboards_definitions` on first boot and resolvable via
  `rubix.dashboard.get`. Two KPIs + 3 line charts, currently
  authored with `Static` placeholder sources.

## What was NOT verified live (success bar items 2–4)

### Item 2: 30-day window < 1.5s wall-clock
Cannot measure until charts source from L3. Today the JSON ships
`Static.points = []`, so the chart resolver never hits CH.

### Item 3: ≤ 9000 rows total across the 3 meter charts
Same blocker — no L3 round trip from the chart resolver yet.

### Item 4: Zoom 6h → L2 cross-over
Stage doc already calls this gated by zoom-driven re-query support
on the chart node-kind. No evidence such a path exists yet.

## Why the chart resolver isn't wired

`ChartSource` (see `crates/starter-ui-ir/src/chart.rs`) exposes
`Series` / `SeriesByKind` / `Rows` / `SeriesFromRsql` / `Static`.
None of those variants name a `rubix.analytics.query` template;
`Rows` / `SeriesFromRsql` take an RSQL filter expression against
the engine's slot store, not a parameterised ClickHouse template.

Bridging the two needs a design call:

1. Add a `ChartSource::AnalyticsTemplate { name, params }` variant
   + a resolver branch that calls `AnalyticsQueryTool::invoke`,
   maps the rows to `(ts_ms, value)` points, and emits the chart
   payload. Smallest change; couples the SDUI resolver to the
   analytics tool.
2. Materialise L3 buckets into the slot store (one slot per
   meter) and let charts use the existing `Series` variant. Bigger
   write path but reuses the live-update wire and zoom support.

Either way the work is its own session — the L3 plumbing this
stage delivered is the prerequisite, not part of the dashboard
node-kind change.

## Suggested next steps (in order)

1. Decide between options 1 and 2 above. Likely (1) for stage-05
   scope, (2) only if Series-style live updates are wanted.
2. Add the variant + resolver branch, then update
   `dashboards/data-flow-site-a.json` to use it for the 3 charts
   and the 2 KPIs.
3. Re-run the live e2e and capture items 2 + 3 (timing + row
   count). Item 4 (zoom) remains a separate follow-up unless the
   chosen node-kind supports it natively.

## Evidence captured this stage

```
# e2e run #2 (post cold restart)
rollup_15m → rows=12, lookback_minutes=60
SELECT count(), uniqExact(meter_id), max(bucket_start)
  FROM rubix.meter_readings_15m
→ 12  3  2026-05-26 04:45:00

rubix.dashboard.get { tenant_id:"system",
                      page_id:"dashboard.data-flow-site-a" }
→ revision_id=7bbb9f59-c81a-4cf9-b71f-5330a3ba5f95
  title="Data flow — Site A"

rubix.analytics.query meter_kwh_last_24h    {tenant_id:"site-a"}
→ row_count=2  (elec.main + elec.hvac)
rubix.analytics.query meter_litres_last_24h {tenant_id:"site-a"}
→ row_count=1  (water.main)
```
