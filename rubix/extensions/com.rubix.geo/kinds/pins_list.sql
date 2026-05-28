-- com.rubix.geo.pins_list
-- Every pin the caller's tenant can see.
SELECT pin_id, layer_id, name, description,
       lng, lat, geometry_type, geometry,
       icon, color, actions, props,
       created_at, updated_at
FROM   com_rubix_geo__pins
WHERE  tenant_id = $caller_tenant_id
ORDER  BY created_at DESC
LIMIT  $limit;
