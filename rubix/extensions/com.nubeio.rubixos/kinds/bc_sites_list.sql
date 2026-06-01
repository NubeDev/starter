-- com.nubeio.rubixos.bc_sites_list
--
-- Sites in the provisioning topology. Tenant-scoped via
-- $caller_tenant_id (bound by the host's WarehouseReadHandle).
SELECT site_id,
       name,
       created_at
FROM   com_nubeio_rubixos__bc_sites
WHERE  tenant_id = $caller_tenant_id
ORDER  BY name NULLS LAST
LIMIT  $limit;
