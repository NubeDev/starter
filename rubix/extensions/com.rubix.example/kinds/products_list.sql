-- com.rubix.example.products_list
--
-- Full product rows for the bundled CRUD panel. The host's generic
-- template compiler binds $caller_tenant_id from the call context
-- and $limit from the validated params.
SELECT internal_id,
       name,
       brand,
       category,
       price,
       currency,
       stock,
       availability,
       color,
       size,
       ingested_at
FROM   com_rubix_example__products
WHERE  tenant_id = $caller_tenant_id
ORDER  BY ingested_at DESC NULLS LAST, internal_id ASC
LIMIT  $limit;
