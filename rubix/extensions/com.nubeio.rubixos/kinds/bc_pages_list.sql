-- com.nubeio.rubixos.bc_pages_list
--
-- Dashboard pages. Tenant-scoped via $caller_tenant_id (bound by
-- the host's WarehouseReadHandle).
SELECT page_id,
       name,
       created_at
FROM   com_nubeio_rubixos__bc_pages
WHERE  tenant_id = $caller_tenant_id
ORDER  BY name NULLS LAST
LIMIT  $limit;
