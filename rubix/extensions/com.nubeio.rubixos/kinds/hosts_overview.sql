-- com.nubeio.rubixos.hosts_overview
--
-- Top-of-tree summary: one row per Rubix-OS edge host (a Pi or
-- similar). The dashboard KPI cards bind to this.
SELECT host_uuid,
       MAX(host_name)        AS host_name,
       MAX(host_description) AS host_description,
       count(DISTINCT network_uuid) AS network_count,
       count(DISTINCT device_uuid)  AS device_count,
       count(*)                     AS point_count
FROM   com_nubeio_rubixos__points
WHERE  tenant_id = $caller_tenant_id
GROUP  BY host_uuid
ORDER  BY point_count DESC, host_name NULLS LAST
LIMIT  $limit;
