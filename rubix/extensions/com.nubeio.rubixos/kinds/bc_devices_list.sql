-- com.nubeio.rubixos.bc_devices_list
--
-- Provisioned devices, optionally filtered by site and/or status.
-- Tenant-scoped via $caller_tenant_id (bound by the host's
-- WarehouseReadHandle). Empty string disables either filter.
SELECT device_id,
       template,
       name,
       network,
       address,
       site_id,
       location_id,
       page_id,
       status,
       provisioned_at
FROM   com_nubeio_rubixos__bc_devices
WHERE  tenant_id = $caller_tenant_id
  AND  ($site_id = '' OR site_id = $site_id)
  AND  ($status  = '' OR status  = $status)
ORDER  BY site_id NULLS LAST, device_id
LIMIT  $limit;
