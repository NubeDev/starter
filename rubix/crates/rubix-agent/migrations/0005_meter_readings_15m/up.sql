-- L3 dashboard mart for the rubix data-flow scenario.
--
-- Materialised by the `rubix.warehouse.rollup_15m` tool (called
-- once every 5 minutes by the bundled `com.rubix.data-flow.rollup`
-- flow). Each row is a 15-minute bucket per (tenant_id, meter_id)
-- carrying min / avg / max across the bucket plus a `quality_mix`
-- map counting how many L2 rows fell into each quality class.
--
-- 30 days × 3 meters × 96 buckets/day ≈ 8640 rows, which is the
-- whole point of stage 05: server-side downsampling so the
-- dashboard never streams 130k L2 rows to the browser. See
-- `rubix/docs/sessions/data-flow/05-dashboard-at-scale.md`.
--
-- Schema is **owned by warehouse-bundled migration**, same
-- precedent as `0003_meter_readings_raw` and `0004_meter_readings_1m`.
-- The stage doc's call to `rubix.clickhouse.mart.create` is the
-- user-facing path; the core mart lands at boot via this migration
-- so a cold start always has the table without operator intervention.
--
-- ENGINE = ReplacingMergeTree so re-running the rollup for the
-- same bucket idempotently supersedes the prior row (rollup runs
-- with a 30-minute lookback window; later passes refine earlier
-- buckets when more L2 data has arrived).
--
-- TTL `bucket_start + INTERVAL 730 DAY` — two years of cleaned-
-- and-downsampled data per the warehouse design doc's L1<L2<L3
-- retention rule (L1 is 14 days, L2 is 180 days).
CREATE TABLE IF NOT EXISTS meter_readings_15m (
    tenant_id    String,
    meter_id     String,
    kind         LowCardinality(String),
    unit         LowCardinality(String),
    bucket_start DateTime,
    value_avg    Nullable(Float64),
    value_min    Nullable(Float64),
    value_max    Nullable(Float64),
    quality_mix  Map(LowCardinality(String), UInt32)
)
ENGINE = ReplacingMergeTree
PARTITION BY toYYYYMM(bucket_start)
ORDER BY (tenant_id, meter_id, bucket_start)
TTL bucket_start + INTERVAL 730 DAY;
