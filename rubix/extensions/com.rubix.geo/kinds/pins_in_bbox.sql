-- com.rubix.geo.pins_in_bbox
-- v1: plain BETWEEN. Add PostGIS + GiST index for large datasets.
SELECT pin_id, layer_id, name, description,
       lng, lat, geometry_type, geometry,
       icon, color, actions, props,
       created_at, updated_at
FROM   com_rubix_geo__pins
WHERE  tenant_id = $caller_tenant_id
  AND  lng BETWEEN $min_lng AND $max_lng
  AND  lat BETWEEN $min_lat AND $max_lat
ORDER  BY created_at DESC
LIMIT  $limit;
