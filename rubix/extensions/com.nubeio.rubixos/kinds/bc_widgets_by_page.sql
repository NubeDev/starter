-- com.nubeio.rubixos.bc_widgets_by_page
--
-- Widgets placed on one dashboard page. Tenant-scoped via
-- $caller_tenant_id (bound by the host's WarehouseReadHandle).
SELECT widget_id,
       page_id,
       device_id,
       point_id,
       widget,
       slot,
       role,
       title
FROM   com_nubeio_rubixos__bc_widgets
WHERE  tenant_id = $caller_tenant_id
  AND  page_id = $page_id
ORDER  BY device_id, slot;
