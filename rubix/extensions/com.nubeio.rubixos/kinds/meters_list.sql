-- com.nubeio.rubixos.meters_list
--
-- Lists meter points filtered by a high-level `kind` tag
-- (`elec` or `water`) plus an optional `secondary_tag`. A "meter"
-- here is any point that carries the `meter` point-tag and the
-- requested kind tag in `com_nubeio_rubixos__point_tags`. The
-- secondary tag (e.g. `power`, `energy`, `usage`, `reading`) is
-- how the BMS distinguishes the various channels each physical
-- meter exposes; pass empty string to skip that filter.
--
-- `unit` is pulled from `point_meta_tags` (key `unit`) which is
-- the engineering unit recorded on the point (`kW`, `kWh`, `A`,
-- `V`, `litres`, `kL`, …).
SELECT p.uuid,
       p.name,
       p.device_uuid,
       p.device_name,
       p.network_uuid,
       p.network_name,
       p.host_uuid,
       p.host_name,
       MAX(u.value) AS unit
FROM   com_nubeio_rubixos__points p
JOIN   com_nubeio_rubixos__point_tags tm
       ON tm.tenant_id = p.tenant_id
      AND tm.point_uuid = p.uuid
      AND tm.host_uuid  = p.host_uuid
      AND tm.tag        = 'meter'
JOIN   com_nubeio_rubixos__point_tags tk
       ON tk.tenant_id = p.tenant_id
      AND tk.point_uuid = p.uuid
      AND tk.host_uuid  = p.host_uuid
      AND tk.tag        = $kind
LEFT JOIN com_nubeio_rubixos__point_meta_tags u
       ON u.tenant_id = p.tenant_id
      AND u.point_uuid = p.uuid
      AND u.host_uuid  = p.host_uuid
      AND u.key        = 'unit'
WHERE  p.tenant_id = $caller_tenant_id
  AND  ($secondary_tag = ''
        OR EXISTS (
          SELECT 1
          FROM   com_nubeio_rubixos__point_tags ts
          WHERE  ts.tenant_id = p.tenant_id
            AND  ts.point_uuid = p.uuid
            AND  ts.host_uuid  = p.host_uuid
            AND  ts.tag        = $secondary_tag
        ))
GROUP  BY p.uuid, p.name, p.device_uuid, p.device_name,
         p.network_uuid, p.network_name, p.host_uuid, p.host_name
ORDER  BY p.host_name NULLS LAST,
         p.network_name NULLS LAST,
         p.device_name NULLS LAST,
         p.name NULLS LAST
LIMIT  $limit;
