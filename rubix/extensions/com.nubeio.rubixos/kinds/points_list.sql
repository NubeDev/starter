-- com.nubeio.rubixos.points_list
--
-- Paginated catalog read. Tenant-scoped via $caller_tenant_id (bound
-- by the host's WarehouseReadHandle from the operator session).
SELECT uuid,
       name,
       description,
       device_uuid,
       device_name,
       network_uuid,
       network_name,
       host_uuid,
       host_name
FROM   com_nubeio_rubixos__points
WHERE  tenant_id = $caller_tenant_id
ORDER  BY host_uuid, network_name NULLS LAST, device_name NULLS LAST, name NULLS LAST
LIMIT  $limit
OFFSET $offset;
