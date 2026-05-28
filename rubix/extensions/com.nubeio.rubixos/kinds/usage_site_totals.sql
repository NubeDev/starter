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
WITH per_point AS (
    SELECT h.host_uuid,
           h.point_uuid,
           AVG(h.value)::float8 AS avg_value,
           COUNT(*)             AS sample_count
    FROM   com_nubeio_rubixos__histories h
    WHERE  h.tenant_id  = $caller_tenant_id
      AND  h.point_uuid = ANY (string_to_array($point_uuids, ','))
      AND  h."timestamp" >= $from::timestamptz
      AND  h."timestamp" <  $to::timestamptz
    GROUP  BY h.host_uuid, h.point_uuid
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
