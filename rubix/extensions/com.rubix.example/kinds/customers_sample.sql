-- com.rubix.example.customers_sample
--
-- Audit-only SQL body for the customers-sample template. Returns
-- up to `$limit` raw customer rows for the caller's tenant so the
-- bundled UI can show the data-quality rule preview against live
-- warehouse data instead of bundled JSON.
SELECT customer_id,
       first_name,
       last_name,
       company,
       city,
       country,
       email,
       subscription_date,
       website
FROM   com_rubix_example__customers
WHERE  tenant_id = $caller_tenant_id
ORDER  BY ingested_at DESC NULLS LAST, customer_id ASC
LIMIT  $limit;
