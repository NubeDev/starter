-- com.nubeio.rubixos.bc_points_by_device
--
-- Points materialised for one device. Tenant-scoped via
-- $caller_tenant_id (bound by the host's WarehouseReadHandle).
SELECT point_id,
       device_id,
       point_key,
       name,
       unit,
       kind,
       widget,
       writable,
       trend_on,
       alarm_on,
       trend_interval
FROM   com_nubeio_rubixos__bc_points
WHERE  tenant_id = $caller_tenant_id
  AND  device_id = $device_id
ORDER  BY point_key;
