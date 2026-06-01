-- com.nubeio.rubixos.bc_pages_list
--
-- Dashboard pages, optionally filtered by site. A page belongs to a
-- site (its dashboard), so the client browses "Site → its pages".
-- Empty/absent `$site_id` returns every page (incl. legacy pages with
-- no site). Tenant-scoped via $caller_tenant_id (bound by the host's
-- WarehouseReadHandle).
SELECT page_id,
       site_id,
       name,
       created_at
FROM   com_nubeio_rubixos__bc_pages
WHERE  tenant_id = $caller_tenant_id
  AND  ($site_id = '' OR site_id = $site_id)
ORDER  BY name NULLS LAST
LIMIT  $limit;
