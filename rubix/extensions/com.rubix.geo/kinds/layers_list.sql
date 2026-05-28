-- com.rubix.geo.layers_list
SELECT layer_id, name, description, style_url,
       visible, min_zoom, max_zoom, color, cluster, sort_order,
       created_at
FROM   com_rubix_geo__map_layers
WHERE  tenant_id = $caller_tenant_id
ORDER  BY sort_order ASC, name ASC
LIMIT  $limit;
