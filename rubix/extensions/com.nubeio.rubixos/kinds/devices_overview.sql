-- com.nubeio.rubixos.devices_overview
--
-- One row per device with the number of contributing points. Useful
-- for the dashboard's "devices" table.
SELECT device_uuid,
       MAX(device_name)        AS device_name,
       MAX(device_description)  AS device_description,
       MAX(network_uuid)        AS network_uuid,
       MAX(network_name)        AS network_name,
       MAX(host_uuid)           AS host_uuid,
       MAX(host_name)           AS host_name,
       count(*)                 AS point_count
FROM   com_nubeio_rubixos__points
WHERE  tenant_id = $caller_tenant_id
  AND  device_uuid IS NOT NULL
GROUP  BY device_uuid
ORDER  BY point_count DESC, device_name NULLS LAST
LIMIT  $limit;
