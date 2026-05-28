-- com.nubeio.rubixos.usage_per_meter
--
-- Per-point AVG / MIN / MAX over a window, for an arbitrary set
-- of meter point_uuids. Used by the dashboard's "Top meters"
-- leaderboard and the per-site sparkline strip. Splits like the
-- usage_site_totals template but at point granularity (no
-- secondary aggregation).
SELECT h.point_uuid,
       MAX(p.name)           AS name,
       MAX(p.host_uuid)      AS host_uuid,
       MAX(p.host_name)      AS host_name,
       MAX(p.device_name)    AS device_name,
       AVG(h.value)::float8  AS avg_value,
       MIN(h.value)::float8  AS min_value,
       MAX(h.value)::float8  AS max_value,
       COUNT(*)              AS sample_count
FROM   com_nubeio_rubixos__histories h
LEFT   JOIN com_nubeio_rubixos__points p
       ON p.tenant_id  = h.tenant_id
      AND p.host_uuid  = h.host_uuid
      AND p.uuid       = h.point_uuid
WHERE  h.tenant_id   = $caller_tenant_id
  AND  h.point_uuid  = ANY (string_to_array($point_uuids, ','))
  AND  h."timestamp" >= $from::timestamptz
  AND  h."timestamp" <  $to::timestamptz
GROUP  BY h.point_uuid
ORDER  BY avg_value DESC NULLS LAST
LIMIT  $limit;
