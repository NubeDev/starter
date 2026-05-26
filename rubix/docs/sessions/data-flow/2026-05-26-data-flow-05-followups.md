# Stage 05 follow-ups — "no data" on /dashboards/data-flow-site-a

**Stage status:** ✅ landed (PROGRESS row 5) for the L3 + rollup +
analytics path. The dashboard page itself **renders but shows
zeros / "no data"** in the browser. This note tells the next
session exactly why, and what to change.

Compare:

- ✅ `/dashboards/disk-overview` — KPIs + chart populated.
- ❌ `/dashboards/data-flow-site-a` — KPIs `0 kWh` / `0 L`, all
  three charts say "no data".

The L3 mart, the rollup tool, the rollup flow, the analytics
templates and the dashboard JSON all work end-to-end. The browser
shows zeros because **the dashboard JSON deliberately ships
placeholder `Static` sources** — that was the only shape this
stage could land without first extending the SDUI chart resolver.

## Exact reason it's empty

[rubix/crates/rubix-flows/dashboards/data-flow-site-a.json](../../../crates/rubix-flows/dashboards/data-flow-site-a.json)
authors:

```json
"source": { "type": "static", "points": [[0, 0.0]] }   // KPIs
"sources": [{ "type": "static", "points": [] }]        // charts
```

The chart resolver receives those literally. It does not call
`rubix.analytics.query`, does not touch ClickHouse, so the
browser paints exactly what's in the JSON — zero.

`disk-overview` "works" the same way — its `Static.points` are
hard-coded demo numbers. It is not actually live either; the
contrast you see is just "fake numbers vs zeroed-out numbers".

## What's needed to make it real

The `ChartSource` enum in
[crates/starter-ui-ir/src/chart.rs](../../../../crates/starter-ui-ir/src/chart.rs)
exposes `Series` / `SeriesByKind` / `Rows` / `SeriesFromRsql` /
`Static`. **None** of them name a `rubix.analytics.query`
template. The resolver therefore has no path from a chart payload
to the L3 mart this stage built.

To make `/dashboards/data-flow-site-a` show live numbers, do the
following — in this order:

### Step 1 — add the `AnalyticsTemplate` ChartSource variant

In [crates/starter-ui-ir/src/chart.rs](../../../../crates/starter-ui-ir/src/chart.rs)
extend `ChartSource` with:

```rust
AnalyticsTemplate {
    /// Filename stem under
    /// `rubix-tools/src/analytics/templates/`.
    name: String,
    /// CH `{name:Type}` params bound through the query verb.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    params: std::collections::BTreeMap<String, serde_json::Value>,
    /// Row-shape mapping. For KPIs: which row field becomes the
    /// scalar (`kwh`, `litres`). For line charts: which field is
    /// the bucket timestamp + which is the value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    map: Option<AnalyticsTemplateMap>,
},
```

`AnalyticsTemplateMap` is the small DTO the resolver needs to
convert the `rubix.analytics.query` row payload into `(ts_ms, value)`
points (or a single KPI scalar). Two-field struct: `value_field`
+ optional `ts_field`.

### Step 2 — resolver branch

Wherever the SDUI resolver currently honours `Static` /
`Series` (look for the `match` over `ChartSource` inside
`starter-ui-ir` / the SDUI resolver module — `Explore` for
`ChartSource::Static =>`), add an `AnalyticsTemplate` arm that:

1. Calls `AnalyticsQueryTool::invoke({ name, params })` through
   the same in-process tool registry the dashboard.get uses.
2. Walks `response.rows` and emits `(row[ts_field], row[value_field])`
   points (or, for KPIs, the scalar from row 0).
3. Falls back to an empty payload on error so the chart paints
   "no data" rather than freezing.

### Step 3 — update `data-flow-site-a.json`

Replace the placeholder `Static` sources with the new variant.
Two new templates **must** be added (24h is fine for KPIs but
the 30-day chart needs its own template — the existing two only
do 24h aggregates):

- `meter_value_30d_15m.sql` — `meter_id, bucket_start, value_avg`
  from `rubix.meter_readings_15m` where
  `bucket_start >= now() - INTERVAL 30 DAY` and
  `meter_id = {meter_id:String}`. (One row per 15-min bucket.)

Then the JSON looks like (KPI shown — the same shape works for
charts):

```json
{
  "type": "kpi",
  "id": "kpi-kwh-24h",
  "label": "Site A — last 24h kWh",
  "format": "number",
  "unit_symbol": "kWh",
  "source": {
    "type": "analytics_template",
    "name": "meter_kwh_last_24h",
    "params": { "tenant_id": "site-a" },
    "map": { "value_field": "kwh" }
  }
}
```

And per-meter charts:

```json
"sources": [{
  "type": "analytics_template",
  "name": "meter_value_30d_15m",
  "params": { "tenant_id": "site-a", "meter_id": "site-a.elec.main" },
  "map": { "ts_field": "bucket_start", "value_field": "value_avg" }
}]
```

### Step 4 — tenant_id in the resolved page

The dashboard is seeded under `BUNDLED_TENANT = "system"` but the
L3 data is `tenant_id = "site-a"`. The JSON above hard-codes
`"site-a"` in params. That's fine for a single-tenant demo; if
this ever needs to follow the viewer's tenant, swap the literal
for a resolver-side placeholder (the SDUI resolver already
substitutes `{tenant_id}` style tokens — confirm before relying
on it).

## How to verify after the change

```bash
# 1. boot
cd rubix && make restart && sleep 8 && grep listening /tmp/rubix-agent.log

# 2. seed L2 (stages 02 + 03 already do this); drive a rollup
#    to make sure L3 has rows in the last 24h:
curl -s -b /tmp/smoke-cookies.txt -H "x-csrf-token: $CSRF" -X POST \
  'http://127.0.0.1:8088/api/v1/tools/rubix.warehouse.rollup_15m' \
  -H 'content-type: application/json' -d '{"lookback_minutes":1440}'

# 3. confirm L3 is populated
curl -s -X POST -d \
  "SELECT count(), uniqExact(meter_id), max(bucket_start) FROM rubix.meter_readings_15m" \
  http://127.0.0.1:8124/

# 4. open the page — KPIs should match
curl -s -b /tmp/smoke-cookies.txt -H "x-csrf-token: $CSRF" -X POST \
  'http://127.0.0.1:8088/api/v1/tools/rubix.analytics.query' \
  -H 'content-type: application/json' \
  -d '{"name":"meter_kwh_last_24h","params":{"tenant_id":"site-a"}}'

# 5. browser → http://127.0.0.1:5173/dashboards/data-flow-site-a
#    KPIs should match step 4; the three charts should plot
#    `value_avg` per meter over the last 30 days (≤ 30×96 = 2880
#    points per chart, ≤ 9000 total — that satisfies success bar #3).
```

## Don't get distracted by these

- **The dashboard is not in PG.** It is. The page row exists under
  `tenant_id=system, page_id=dashboard.data-flow-site-a`, seeded
  on first boot by `boot/dashboards_seed.rs`. Verify:
  `PGPASSWORD=rubix-dev psql -h 127.0.0.1 -p 5433 -U rubix -d rubix
  -c "SELECT page_id FROM dashboards_definitions WHERE
  superseded_at IS NULL;"`
- **The rollup didn't run.** It does. `rubix.warehouse.rollup_15m`
  via curl returns `rows=N` and CH `count() FROM
  rubix.meter_readings_15m` matches. The rollup flow
  `com.rubix.data-flow.rollup` fires every 5 minutes.
- **`disk-overview` is wired and `data-flow-site-a` is broken.**
  Neither is wired to a live source — both ship `Static`
  points. `disk-overview` just ships **non-zero** static points,
  so it looks alive; the data-flow JSON ships zeros and empties.
- **The catalogue keys are missing.** The two new keys
  (`rubix.warehouse.rolled_up`, `rubix.warehouse.rollup.empty`)
  are present in both `en.json` and `es.json`.

## Evidence captured this stage (still valid)

```
# e2e run #2 (post cold restart)
rubix.warehouse.rollup_15m → rows=12, lookback_minutes=60
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

Everything below the SDUI chart resolver is live and correct.
The only gap is the resolver ↔ analytics bridge described above.
