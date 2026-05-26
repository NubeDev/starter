-- L2 cleaned-mart for the rubix data-flow scenario.
--
-- Materialised by the `rubix.warehouse.clean_minute` tool (called
-- once per minute by the bundled `com.rubix.data-flow.cleaner`
-- flow). Each row is a 1-minute bucket per (tenant_id, meter_id);
-- gaps are surfaced as `value = NULL, quality = 'missing'` rather
-- than as absent rows so downstream dashboards / rules can detect
-- "no data" without joining a calendar.
--
-- Schema is **owned by warehouse-bundled migration**, same
-- precedent as `0003_meter_readings_raw`. The stage doc's call to
-- `rubix.clickhouse.mart.create` is the user-facing path; the
-- core mart lands at boot via this migration so a cold start
-- always has the table without operator intervention. See
-- `rubix/docs/sessions/data-flow/03-clean-to-l2.md`.
--
-- ENGINE = ReplacingMergeTree so re-running the cleaner for the
-- same bucket idempotently supersedes the prior row (cleaner runs
-- with a 5-minute lookback window; later passes refine earlier
-- buckets when more L1 data has arrived).
--
-- TTL `bucket_start + INTERVAL 180 DAY` — months of cleaned data
-- per the warehouse design doc's L1<L2<L3 retention rule (L1 is
-- 14 days, L3 will be 730 days).
CREATE TABLE IF NOT EXISTS meter_readings_1m (
    tenant_id    String,
    meter_id     String,
    kind         LowCardinality(String),
    unit         LowCardinality(String),
    bucket_start DateTime,
    value        Nullable(Float64),
    quality      LowCardinality(String)
)
ENGINE = ReplacingMergeTree
PARTITION BY toYYYYMM(bucket_start)
ORDER BY (tenant_id, meter_id, bucket_start)
TTL bucket_start + INTERVAL 180 DAY;
