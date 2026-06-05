-- com.nubeio.rubixos.usage_site_totals
--
-- Aggregate "usage" per site (host) for a set of meter point_uuids
-- and a time window. Inner CTE averages each point individually
-- so an unevenly-sampled meter doesn't bias the host total; the
-- outer SUM then rolls per-point averages up to a host total
-- (interpreted as "total average power" for kW channels, or "sum
-- of average readings" for cumulative meters).
--
-- `point_uuids` is a comma-separated string (the warehouse template
-- bridge binds JSON arrays as TEXT, so we explode via
-- `string_to_array`). The host JOIN re-attaches a name for the UI.
--
-- Reads the `com_nubeio_rubixos__histories_1m` continuous aggregate
-- (1-minute pre-rollup) instead of the raw ~955M-row hypertable. The
-- per-point AVG is reconstructed as a sample-count-weighted mean of
-- the per-minute averages — identical to `AVG(value)` over the raw
-- rows. The cagg's `(tenant_id, point_uuid, bucket)` index makes the
-- point-set + window filter an index seek (~5x faster than raw on the
-- adopted DB; see PRODUCTION.md). Install/refresh via
-- `scripts/post-load.sql` / `examples/refresh_histories_1m.rs`.
WITH per_point AS (
    SELECT c.host_uuid,
           c.point_uuid,
           (SUM(c.avg_value * c.sample_count)
              / NULLIF(SUM(c.sample_count), 0))::float8 AS avg_value,
           SUM(c.sample_count)                          AS sample_count
    FROM   com_nubeio_rubixos__histories_1m c
    WHERE  c.tenant_id  = $caller_tenant_id
      AND  c.point_uuid = ANY (string_to_array($point_uuids, ','))
      AND  c.bucket >= $from::timestamptz
      AND  c.bucket <  $to::timestamptz
    GROUP  BY c.host_uuid, c.point_uuid
)
SELECT pp.host_uuid,
       MAX(p.host_name)              AS host_name,
       SUM(pp.avg_value)::float8     AS total_value,
       COUNT(DISTINCT pp.point_uuid) AS point_count,
       SUM(pp.sample_count)::int8    AS sample_count
FROM   per_point pp
LEFT   JOIN com_nubeio_rubixos__points p
       ON p.tenant_id = $caller_tenant_id
      AND p.host_uuid = pp.host_uuid
GROUP  BY pp.host_uuid
ORDER  BY total_value DESC NULLS LAST;
