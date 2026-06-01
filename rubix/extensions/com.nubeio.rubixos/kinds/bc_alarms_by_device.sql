-- com.nubeio.rubixos.bc_alarms_by_device
--
-- Alarm rules materialised for one device. Tenant-scoped via
-- $caller_tenant_id (bound by the host's WarehouseReadHandle).
SELECT alarm_id,
       device_id,
       point_id,
       point_key,
       predicate,
       severity,
       message,
       enabled
FROM   com_nubeio_rubixos__bc_alarms
WHERE  tenant_id = $caller_tenant_id
  AND  device_id = $device_id
ORDER  BY point_key;
