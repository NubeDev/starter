-- com.nubeio.rubixos.networks_overview
SELECT network_uuid,
       MAX(network_name)        AS network_name,
       MAX(network_description) AS network_description,
       MAX(host_uuid)           AS host_uuid,
       MAX(host_name)           AS host_name,
       count(DISTINCT device_uuid) AS device_count,
       count(*)                    AS point_count
FROM   com_nubeio_rubixos__points
WHERE  tenant_id = $caller_tenant_id
  AND  network_uuid IS NOT NULL
GROUP  BY network_uuid
ORDER  BY point_count DESC, network_name NULLS LAST
LIMIT  $limit;
