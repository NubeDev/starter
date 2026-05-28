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
SELECT time_bucket($bucket::interval, h."timestamp") AS bucket,
       h.host_uuid,
       AVG(h.value)::float8 AS avg_value,
       COUNT(*)             AS sample_count
FROM   com_nubeio_rubixos__histories h
WHERE  h.tenant_id  = $caller_tenant_id
  AND  h.point_uuid = ANY (string_to_array($point_uuids, ','))
  AND  h."timestamp" >= $from::timestamptz
  AND  h."timestamp" <  $to::timestamptz
GROUP  BY bucket, h.host_uuid
ORDER  BY bucket, h.host_uuid;
