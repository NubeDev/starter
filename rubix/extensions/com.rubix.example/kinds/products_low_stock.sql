-- com.rubix.example.products_low_stock
--
-- Audit-only SQL body for the products low-stock template.
SELECT internal_id,
       name,
       brand,
       category,
       stock,
       availability,
       price,
       currency
FROM   com_rubix_example__products
WHERE  tenant_id = $caller_tenant_id
  AND  stock < $threshold
ORDER  BY stock ASC, name ASC
LIMIT  $limit;
