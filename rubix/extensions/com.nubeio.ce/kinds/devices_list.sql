-- com.nubeio.ce.devices_list
--
-- Registered control engines, optionally filtered by status.
-- Tenant-scoped via $caller_tenant_id (bound by the host's
-- WarehouseReadHandle). Empty string disables the status filter.
-- `password` is intentionally NOT selected — list reads never expose
-- the secret to the browser.
SELECT device_id,
       name,
       description,
       engine_kind,
       ip,
       port,
       username,
       status,
       created_at,
       updated_at
FROM   com_nubeio_ce__ce_devices
WHERE  tenant_id = $caller_tenant_id
  AND  ($status = '' OR status = $status)
ORDER  BY name NULLS LAST, device_id
LIMIT  $limit;
