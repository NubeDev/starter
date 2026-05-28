-- com.nubeio.rubixos.points_by_device
SELECT uuid,
       name,
       description,
       network_uuid,
       network_name,
       host_uuid,
       host_name
FROM   com_nubeio_rubixos__points
WHERE  tenant_id   = $caller_tenant_id
  AND  device_uuid = $device_uuid
ORDER  BY name NULLS LAST
LIMIT  $limit;
