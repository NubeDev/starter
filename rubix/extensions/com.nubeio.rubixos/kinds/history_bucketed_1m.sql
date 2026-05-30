-- com.nubeio.rubixos.history_bucketed_1m
--
-- Rollup variant of `history_bucketed` sourced from the
-- `com_nubeio_rubixos__histories_1m` continuous aggregate. A
-- 24-hour chart of one point hits ~1,440 pre-aggregated rows
-- instead of potentially millions of raw samples.
--
-- The CAGG is installed by `scripts/post-load.sql`; if it does
-- not exist yet (fresh DB, load-dump.sh not run), this template
-- fails with a "relation does not exist" error — that is the
-- correct shape (panels render no-data and surface the error in
-- diagnostics, rather than silently scanning the raw hypertable).
--
-- The `$bucket` param is accepted for caller compatibility with
-- `history_bucketed` but is currently unused: the CAGG is fixed
-- at 1-minute granularity. A future `history_bucketed_1m` could
-- re-bucket to a wider interval on top of the CAGG, but that's
-- a separate template (`history_bucketed_5m_over_1m`, etc.).
SELECT bucket,
       min_value,
       max_value,
       avg_value,
       sample_count
FROM   com_nubeio_rubixos__histories_1m
WHERE  tenant_id  = $caller_tenant_id
  AND  point_uuid = $point_uuid
  AND  bucket >= $from::timestamptz
  AND  bucket <  $to::timestamptz
ORDER  BY bucket;
