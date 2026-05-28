-- com.nubeio.rubixos.points_search
--
-- Substring-match across point + device + network names. The host
-- binds $query verbatim (no LIKE wildcards added) — callers wrap
-- with `%…%` themselves if they want fuzzy matching.
SELECT uuid,
       name,
       description,
       device_uuid,
       device_name,
       network_uuid,
       network_name,
       host_uuid,
       host_name
FROM   com_nubeio_rubixos__points
WHERE  tenant_id = $caller_tenant_id
  AND  (name ILIKE $query
        OR description ILIKE $query
        OR device_name ILIKE $query
        OR network_name ILIKE $query)
ORDER  BY name NULLS LAST
LIMIT  $limit;
