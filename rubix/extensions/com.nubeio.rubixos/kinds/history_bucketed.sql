-- com.nubeio.rubixos.history_bucketed
--
-- Timescale `time_bucket()` aggregate. Requires
-- `com_nubeio_rubixos__histories` to be a hypertable — the
-- bundled `scripts/load-dump.sh` does the conversion before
-- bulk-INSERTing the dump.
SELECT time_bucket($bucket::interval, "timestamp") AS bucket,
       min(value)::float8 AS min_value,
       max(value)::float8 AS max_value,
       avg(value)::float8 AS avg_value,
       count(*)           AS sample_count
FROM   com_nubeio_rubixos__histories
WHERE  tenant_id  = $caller_tenant_id
  AND  point_uuid = $point_uuid
  AND  "timestamp" >= $from::timestamptz
  AND  "timestamp" <  $to::timestamptz
GROUP  BY bucket
ORDER  BY bucket;
