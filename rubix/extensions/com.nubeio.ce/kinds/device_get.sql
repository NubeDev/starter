-- com.nubeio.ce.device_get
--
-- One control-engine row by id, INCLUDING connection credentials.
-- Backs the engine REST proxy (the process needs username/password to
-- reach the remote runtime). Tenant-scoped via $caller_tenant_id.
SELECT device_id,
       name,
       description,
       engine_kind,
       ip,
       port,
       username,
       password,
       status,
       created_at,
       updated_at
FROM   com_nubeio_ce__ce_devices
WHERE  tenant_id = $caller_tenant_id
  AND  device_id = $device_id
LIMIT  1;
