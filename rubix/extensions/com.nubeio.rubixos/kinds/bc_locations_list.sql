-- com.nubeio.rubixos.bc_locations_list
--
-- Locations, optionally narrowed to one site. Tenant-scoped via
-- $caller_tenant_id (bound by the host's WarehouseReadHandle).
-- Pass empty string for $site_id to list every location.
SELECT location_id,
       site_id,
       name,
       created_at
FROM   com_nubeio_rubixos__bc_locations
WHERE  tenant_id = $caller_tenant_id
  AND  ($site_id = '' OR site_id = $site_id)
ORDER  BY name NULLS LAST
LIMIT  $limit;
