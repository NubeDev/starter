-- com.nubeio.rubixos.bc_pages_list
--
-- Dashboard pages, optionally filtered by site and/or location. A page
-- belongs to a site and (optionally) a location within it, so the
-- client browses "Site → Location → its pages". Empty/absent `$site_id`
-- or `$location_id` widens the corresponding filter (incl. legacy pages
-- with no site/location). Tenant-scoped via $caller_tenant_id (bound by
-- the host's WarehouseReadHandle).
SELECT page_id,
       site_id,
       location_id,
       name,
       created_at
FROM   com_nubeio_rubixos__bc_pages
WHERE  tenant_id = $caller_tenant_id
  AND  ($site_id = '' OR site_id = $site_id)
  AND  ($location_id = '' OR location_id = $location_id)
ORDER  BY name NULLS LAST
LIMIT  $limit;
