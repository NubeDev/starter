-- nexus.core.meter_get
--
-- One meter by id, scoped to the caller's tenant. The tenant predicate is
-- mandatory: it prevents reading a meter belonging to another tenant even if its
-- id is known.
SELECT meter_id,
       name,
       site_id,
       status,
       installed_at
FROM   meters
WHERE  tenant_id = $caller_tenant_id
  AND  meter_id = $meter_id;
