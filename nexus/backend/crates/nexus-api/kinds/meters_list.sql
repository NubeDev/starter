-- nexus.core.meters_list
--
-- Meters in the caller's tenant, optionally filtered by site. Tenant isolation
-- is structural: $caller_tenant_id is host-bound and cannot be supplied by the
-- caller. An empty $site_id disables the site filter.
SELECT meter_id,
       name,
       site_id,
       status
FROM   meters
WHERE  tenant_id = $caller_tenant_id
  AND  ($site_id = '' OR site_id = $site_id)
ORDER  BY site_id NULLS LAST, meter_id
LIMIT  $limit;
