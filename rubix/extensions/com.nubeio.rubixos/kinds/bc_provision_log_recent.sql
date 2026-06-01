-- com.nubeio.rubixos.bc_provision_log_recent
--
-- Recent provisioning audit events, optionally narrowed to one
-- device. Tenant-scoped via $caller_tenant_id (bound by the host's
-- WarehouseReadHandle). Empty string for $device_id lists all.
SELECT event_id,
       device_id,
       event,
       step,
       detail,
       at
FROM   com_nubeio_rubixos__bc_provision_log
WHERE  tenant_id = $caller_tenant_id
  AND  ($device_id = '' OR device_id = $device_id)
ORDER  BY at DESC
LIMIT  $limit;
