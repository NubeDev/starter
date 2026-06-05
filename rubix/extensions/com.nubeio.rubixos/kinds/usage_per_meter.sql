-- com.nubeio.rubixos.usage_per_meter
--
-- Per-point AVG / MIN / MAX over a window, for an arbitrary set
-- of meter point_uuids. Used by the dashboard's "Top meters"
-- leaderboard and the per-site sparkline strip. Splits like the
-- usage_site_totals template but at point granularity (no
-- secondary aggregation).
--
-- Reads the `com_nubeio_rubixos__histories_1m` continuous aggregate
-- (1-minute pre-rollup): per-point avg = sample-count-weighted mean of
-- the per-minute averages, min/max = MIN/MAX of the per-minute
-- extrema — identical to the raw `AVG/MIN/MAX(value)`. The cagg's
-- `(tenant_id, point_uuid, bucket)` index serves the point-set +
-- window filter as an index seek (~5x faster than raw on the adopted
-- DB; see PRODUCTION.md). Install/refresh via `scripts/post-load.sql`
-- / `examples/refresh_histories_1m.rs`.
WITH per_point AS (
    SELECT c.point_uuid,
           c.host_uuid,
           (SUM(c.avg_value * c.sample_count)
              / NULLIF(SUM(c.sample_count), 0))::float8 AS avg_value,
           MIN(c.min_value)::float8                     AS min_value,
           MAX(c.max_value)::float8                     AS max_value,
           SUM(c.sample_count)                          AS sample_count
    FROM   com_nubeio_rubixos__histories_1m c
    WHERE  c.tenant_id  = $caller_tenant_id
      AND  c.point_uuid = ANY (string_to_array($point_uuids, ','))
      AND  c.bucket >= $from::timestamptz
      AND  c.bucket <  $to::timestamptz
    GROUP  BY c.point_uuid, c.host_uuid
)
SELECT pp.point_uuid,
       MAX(p.name)        AS name,
       MAX(pp.host_uuid)  AS host_uuid,
       MAX(p.host_name)   AS host_name,
       MAX(p.device_name) AS device_name,
       AVG(pp.avg_value)::float8 AS avg_value,
       MIN(pp.min_value)::float8 AS min_value,
       MAX(pp.max_value)::float8 AS max_value,
       SUM(pp.sample_count)::int8 AS sample_count
FROM   per_point pp
LEFT   JOIN com_nubeio_rubixos__points p
       ON p.tenant_id = $caller_tenant_id
      AND p.host_uuid = pp.host_uuid
      AND p.uuid      = pp.point_uuid
GROUP  BY pp.point_uuid
ORDER  BY avg_value DESC NULLS LAST
LIMIT  $limit;
