-- com.nubeio.rubixos.bc_device_get
--
-- A single device by `device_id` — the read (R) leg of device CRUD.
-- Returns the full row (incl. `default_ip` and `hw_rev`, which the
-- lighter `bc_devices_list` projection omits) so the detail/"See" view
-- can render the complete identity without listing the whole fleet.
-- Tenant-scoped via $caller_tenant_id (bound by the host's
-- WarehouseReadHandle). Zero or one row.
SELECT device_id,
       template,
       name,
       network,
       address,
       default_ip,
       hw_rev,
       site_id,
       location_id,
       page_id,
       status,
       provisioned_at
FROM   com_nubeio_rubixos__bc_devices
WHERE  tenant_id = $caller_tenant_id
  AND  device_id = $device_id
LIMIT  1;
