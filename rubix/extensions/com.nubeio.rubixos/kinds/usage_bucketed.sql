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
-- `string_to_array`. Requires the histories hypertable created by
-- `scripts/load-dump.sh` for `time_bucket()` to use chunk pruning.
--
-- Fast path: when bucket='1 day' the query reads the
-- `com_nubeio_rubixos__usage_daily_cagg` continuous aggregate
-- instead of the 164M-row raw hypertable. The host's average is
-- reconstructed as a sample-count-weighted mean of the per-point
-- per-day averages, which is mathematically identical to
-- `AVG(h.value) GROUP BY day, host_uuid` on the raw table.
-- Install the CAGG via `scripts/install-caggs.sh` (see DB.md §5.1).
-- Sub-day buckets ('15 minutes' / '1 hour' / '6 hours') keep the
-- raw path — the UNION ALL guard `$bucket = '1 day'` lets PG prune
-- the dead branch at plan time (Result with One-Time Filter).
SELECT bucket, host_uuid, avg_value, sample_count
FROM (
    -- Fast path: bucket = '1 day' → CAGG
    SELECT c.bucket,
           c.host_uuid,
           (SUM(c.avg_value * c.sample_count)
              / NULLIF(SUM(c.sample_count), 0))::float8 AS avg_value,
           SUM(c.sample_count)::int8                    AS sample_count
    FROM   com_nubeio_rubixos__usage_daily_cagg c
    WHERE  $bucket = '1 day'
      AND  c.tenant_id  = $caller_tenant_id
      AND  c.point_uuid = ANY (string_to_array($point_uuids, ','))
      AND  c.bucket >= $from::timestamptz
      AND  c.bucket <  $to::timestamptz
    GROUP  BY c.bucket, c.host_uuid

    UNION ALL

    -- Slow path: sub-day buckets → raw hypertable
    SELECT time_bucket($bucket::interval, h."timestamp") AS bucket,
           h.host_uuid,
           AVG(h.value)::float8 AS avg_value,
           COUNT(*)             AS sample_count
    FROM   com_nubeio_rubixos__histories h
    WHERE  $bucket <> '1 day'
      AND  h.tenant_id  = $caller_tenant_id
      AND  h.point_uuid = ANY (string_to_array($point_uuids, ','))
      AND  h."timestamp" >= $from::timestamptz
      AND  h."timestamp" <  $to::timestamptz
    GROUP  BY time_bucket($bucket::interval, h."timestamp"), h.host_uuid
) x
ORDER  BY bucket, host_uuid;
