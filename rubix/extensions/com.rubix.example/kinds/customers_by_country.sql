-- com.rubix.example.customers_by_country
--
-- Audit-only SQL body captured into TemplateSpec::sql at load time
-- (per row 3 of the extension-north-star: the host integration crate
-- is the resolver; the SPI does not execute this string).
--
-- The host's WarehouseReadHandle backend will bind $caller_tenant_id
-- and $limit / $min_count from the params payload before running.
SELECT country,
       count(*) AS customer_count
FROM   com_rubix_example__customers
WHERE  tenant_id = $caller_tenant_id
GROUP  BY country
HAVING count(*) >= $min_count
ORDER  BY customer_count DESC
LIMIT  $limit;
