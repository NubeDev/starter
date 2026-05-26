# Stage 03 — clean to L2 (normalise, gap-fill, clip spikes, bucketise)

## Scope

**In:** an L2 mart `rubix.meter_readings_1m` that turns the raw L1
stream into uniformly-bucketed, cleaned 1-minute readings ready for
dashboards and rules. Created via `rubix.clickhouse.mart.create`;
materialised periodically by a flow.

**Out:** anomaly detection (stage 04) and dashboard rendering
(stage 05). This stage outputs cleaned data; downstream stages
consume it.

## What "clean" means here

The four mess shapes from [README.md](./README.md), and the
deterministic response to each:

| Mess              | Cleaning rule                                                |
|-------------------|--------------------------------------------------------------|
| Gaps              | Bucket has no L1 row → emit row with `value = NULL`, `quality = 'missing'`. No interpolation in L2 (dashboards / reports decide whether to fill). |
| Spikes (×50)      | If `value > 10 × rolling_median_15m` → keep raw in L1, but write `value = rolling_median_15m`, `quality = 'clipped'` to L2. |
| Stuck zeros       | If `value` is the same exact float for ≥ 5 consecutive buckets AND that value is < 0.01 of meter's 24-h max → write `quality = 'stuck'`, keep the raw value. |
| Jitter / NaN      | Bucket by floor(`epoch_ms` / 60_000). NaN rows → `value = NULL`, `quality = 'nan'`. |

`quality` enum in L2 expands to:
`ok | clipped | stuck | missing | nan`. Each is testable in isolation.

## L2 schema (locked)

```sql
CREATE TABLE rubix.meter_readings_1m (
  tenant_id    String,
  meter_id     String,
  kind         LowCardinality(String),
  unit         LowCardinality(String),
  bucket_start DateTime,                -- floor(epoch_ms / 60_000) * 60
  value        Nullable(Float64),
  quality      LowCardinality(String)   -- ok|clipped|stuck|missing|nan
) ENGINE = ReplacingMergeTree
PARTITION BY toYYYYMM(bucket_start)
ORDER BY (tenant_id, meter_id, bucket_start);
```

`ReplacingMergeTree` so a re-materialisation idempotently overwrites
the same bucket. Retention: **180 days** (L2 holds months, per the
warehouse design doc's L1<L2<L3 rule; L1 is 14 days from stage 02).

## Materialisation strategy

Pick **one** and lock it:

- **A. Periodic flow** — a `com.rubix.data-flow.cleaner` flow with
  a `timer` (every 60 s) firing a `clickhouse-query` node that runs
  the cleaning SQL with a 5-minute lookback window. Simplest;
  preferred.
- **B. ClickHouse materialised view** — a `MATERIALIZED VIEW` on
  `meter_readings_raw` doing the cleaning at insert time. Faster
  but the spike-clip rule needs a rolling window, which a MV cannot
  do cleanly. **Do not pick B** unless A is benchmarked too slow.

The cleaning SQL is one query — write it in a `.sql` file under
`rubix/crates/rubix-flows/flows/queries/` and have the cleaner
flow's `clickhouse-query` node load it. Keep the SQL under 80
lines; if it grows past that, split per `quality` class into
sibling queries that `INSERT` into the same mart.

## Pre-flight

- Stage 02 success bar green — L1 rows are landing with `suspect`
  rows preserved.
- L1 has at least **30 minutes** of data accumulated. The
  rolling-median spike check needs a 15-minute window; let two
  windows go by before judging cleanliness.

## Steps

1. Create the L2 mart:

   ```bash
   curl -s -b /tmp/smoke-cookies.txt -X POST \
     http://127.0.0.1:8088/api/v1/tools/rubix.clickhouse.mart.create \
     -H 'content-type: application/json' \
     -d '{ "mart_name": "meter_readings_1m",
           "body": "CREATE TABLE rubix.meter_readings_1m ( ... ) ENGINE = ReplacingMergeTree ..." }'
   ```

   Heads-up: `mart.create`'s undo path **drops the mart**, losing
   every row materialised after the create. The verb's design doc
   spells this out — see
   [design/clickhouse-rules/README.md §"mart.create undo data-loss caveat"](../../design/clickhouse-rules/README.md#martcreate-undo-data-loss-caveat).
   If you `rubix.undo.last` this stage, expect to re-materialise
   from L1 (which is why L1 retention > materialisation cadence).

2. Set 180-day retention on the L2 mart:

   ```bash
   curl -s -b /tmp/smoke-cookies.txt -X POST \
     http://127.0.0.1:8088/api/v1/tools/rubix.clickhouse.retention.set \
     -H 'content-type: application/json' \
     -d '{ "table_name": "meter_readings_1m", "days": 180 }'
   ```

3. Deploy the cleaner flow (`com.rubix.data-flow.cleaner`) per
   strategy A. Verify with `rubix.flow_ops.list`.

4. Wait 5 minutes (two cleaner ticks past the 15-minute median
   window). Then sample:

   ```bash
   curl -s -X POST -d \
     "SELECT meter_id, quality, count()
        FROM rubix.meter_readings_1m
       WHERE bucket_start >= now() - INTERVAL 30 MINUTE
       GROUP BY meter_id, quality
       ORDER BY meter_id, quality FORMAT TSV" \
     http://127.0.0.1:8124/
   ```

## Success bar

Stage 03 is done when **all four** are true:

1. Every minute in the last 30 minutes has a row per active meter
   (no buckets missing — gaps are present as `quality='missing'`,
   not as absent rows).
2. At least one row per meter has `quality='ok'` (the happy path
   still flows through cleanly).
3. At least one of `clipped`, `stuck`, `nan`, `missing` appears
   for some meter (the producer's mess shows up in L2 as labelled
   rows, not as silently-mangled values).
4. No row has `value > 10 × rolling_median_15m` AND
   `quality='ok'` (the clip rule isn't being skipped).

## If it fails

In order, check:

1. **L2 has no rows at all** — cleaner flow isn't firing. Verify
   with the same flow-listener check from stage 01's "If it
   fails" item 1.
2. **L2 rows but `quality` is always `ok`** — the cleaning SQL
   isn't pattern-matching the mess. Run the SQL by hand against
   L1, slice by slice, and check each `quality` branch fires.
3. **Re-materialisation duplicates rows instead of replacing** —
   the mart isn't `ReplacingMergeTree`, or the `ORDER BY` doesn't
   match the dedup key. Inspect with
   `SHOW CREATE TABLE rubix.meter_readings_1m`.

Write follow-up notes as `YYYY-MM-DD-data-flow-03-clean-<topic>.md`
and stop.

## Decisions taken

- [x] Strategy A (periodic flow)  /  [ ] Strategy B (MV)
- Mart name: `rubix.meter_readings_1m` (do not rename — stages 04, 05 read this)
- L2 retention: 180 days (TTL declared on the table in `0004_meter_readings_1m/up.sql`; the `rubix.clickhouse.retention.set` step in the stage doc is therefore a no-op when the table is created via the bundled migration path — keep it documented for operators authoring marts via `rubix.clickhouse.mart.create`)
- Cleaner flow id: `com.rubix.data-flow.cleaner`
- Cleaning verb: `rubix.warehouse.clean_minute` (one pass per call, 5-minute lookback; idempotent via `ReplacingMergeTree`)
- Stuck-zero detection deferred — needs 5 same-value consecutive buckets; the 60-s producer cadence × 3 meters does not generate enough rows per 5-min cleaner window. Track in a follow-up at stage 04 if rules need it.
