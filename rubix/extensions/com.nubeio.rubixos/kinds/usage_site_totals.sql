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
-- Fast path: when the window is ≥ 2 days the per-point CTE reads
-- the `com_nubeio_rubixos__usage_daily_cagg` continuous aggregate
-- instead of the 164M-row raw hypertable. The host's per-point
-- AVG is reconstructed as a sample-count-weighted mean of the
-- per-day per-point averages, which is mathematically identical
-- to `AVG(h.value) GROUP BY host, point` on the raw table.
-- Install the CAGG via `scripts/install-caggs.sh` (DB.md §5.1).
-- Sub-2-day windows stay on raw — for 24h overlays the CAGG would
-- include the full anchor days, skewing the answer.
WITH per_point_cagg AS (
    SELECT c.host_uuid,
           c.point_uuid,
           (SUM(c.avg_value * c.sample_count)
              / NULLIF(SUM(c.sample_count), 0))::float8 AS avg_value,
           SUM(c.sample_count)                          AS sample_count
    FROM   com_nubeio_rubixos__usage_daily_cagg c
    WHERE  ($to::timestamptz - $from::timestamptz) >= INTERVAL '2 days'
      AND  c.tenant_id  = $caller_tenant_id
      AND  c.point_uuid = ANY (string_to_array($point_uuids, ','))
      AND  c.bucket >= $from::timestamptz
      AND  c.bucket <  $to::timestamptz
    GROUP  BY c.host_uuid, c.point_uuid
),
per_point_raw AS (
    SELECT h.host_uuid,
           h.point_uuid,
           AVG(h.value)::float8 AS avg_value,
           COUNT(*)             AS sample_count
    FROM   com_nubeio_rubixos__histories h
    WHERE  ($to::timestamptz - $from::timestamptz) < INTERVAL '2 days'
      AND  h.tenant_id  = $caller_tenant_id
      AND  h.point_uuid = ANY (string_to_array($point_uuids, ','))
      AND  h."timestamp" >= $from::timestamptz
      AND  h."timestamp" <  $to::timestamptz
    GROUP  BY h.host_uuid, h.point_uuid
),
per_point AS (
    SELECT * FROM per_point_cagg
    UNION ALL
    SELECT * FROM per_point_raw
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
