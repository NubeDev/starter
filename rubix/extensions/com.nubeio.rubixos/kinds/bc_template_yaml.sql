-- com.nubeio.rubixos.bc_template_yaml
--
-- Read one device template including its raw YAML source. The list
-- template (bc_templates_list) deliberately omits the YAML body to
-- keep list reads light; this fetches the full source for the editor
-- and the provisioning engine's strict re-parse. Tenant-scoped via
-- $caller_tenant_id (bound by the host).
SELECT template,
       version,
       display_name,
       network,
       category,
       icon,
       yaml,
       points_json,
       widget_group_json,
       updated_at
FROM   com_nubeio_rubixos__bc_templates
WHERE  tenant_id = $caller_tenant_id
  AND  template  = $template
LIMIT  1;
