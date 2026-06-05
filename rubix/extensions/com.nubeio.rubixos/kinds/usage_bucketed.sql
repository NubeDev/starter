-- com.nubeio.rubixos.usage_bucketed
--
-- Time-bucketed per-host usage series for an arbitrary set of
-- meter point_uuids. Returns one row per (bucket, host_uuid) with
-- the AVG of the underlying point values in that bucket, plus the
-- sample count. The browser pivots this long-form result into
-- one uplot series per host.
--
-- `point_uuids` is a comma-separated string — the warehouse
-- template bridge binds JSON arrays as TEXT, so we split with
-- `string_to_array`.
--
-- Reads the `com_nubeio_rubixos__histories_1m` continuous aggregate
-- (1-minute pre-rollup) and re-buckets it to `$bucket`, instead of
-- scanning the raw ~955M-row hypertable. The cagg's
-- `(tenant_id, point_uuid, bucket)` index turns the point-set +
-- window filter into a real index seek; measured ~5x faster + ~4x
-- less planning than the raw path for a 5000-point / 7-day window
-- on the adopted DB (see PRODUCTION.md). The host's per-bucket
-- average is reconstructed as a sample-count-weighted mean of the
-- per-point per-minute averages — mathematically identical to
-- `AVG(value)` over the raw rows in that bucket.
--
-- All dashboard bucket widths (15 minutes / 1 hour / 6 hours /
-- 1 day) are ≥ the cagg's 1-minute grain, so re-bucketing is exact.
-- The cagg is installed + refreshed by `scripts/post-load.sql`
-- (or, no-`psql`, `examples/refresh_histories_1m.rs`); if it is
-- missing the template errors with "relation does not exist" rather
-- than silently scanning raw — the correct, visible failure mode.
SELECT time_bucket($bucket::interval, c.bucket) AS bucket,
       c.host_uuid,
       (SUM(c.avg_value * c.sample_count)
          / NULLIF(SUM(c.sample_count), 0))::float8 AS avg_value,
       SUM(c.sample_count)::int8                    AS sample_count
FROM   com_nubeio_rubixos__histories_1m c
WHERE  c.tenant_id  = $caller_tenant_id
  AND  c.point_uuid = ANY (string_to_array($point_uuids, ','))
  AND  c.bucket >= $from::timestamptz
  AND  c.bucket <  $to::timestamptz
GROUP  BY time_bucket($bucket::interval, c.bucket), c.host_uuid
ORDER  BY bucket, host_uuid;
