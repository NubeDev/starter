-- com.nubeio.rubixos.bc_templates_list
--
-- Catalog of installed device templates. Tenant-scoped via
-- $caller_tenant_id (bound by the host's WarehouseReadHandle).
SELECT template,
       version,
       display_name,
       network,
       category,
       icon,
       updated_at
FROM   com_nubeio_rubixos__bc_templates
WHERE  tenant_id = $caller_tenant_id
ORDER  BY template
LIMIT  $limit;
