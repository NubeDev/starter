-- nexus.core.top_sites_by_usage
--
-- The top N sites by total usage over the dashboard time range, scoped to the
-- caller's tenant. Tenant isolation is structural: `$caller_tenant_id` is
-- host-bound and mandatory because the data side has no RLS (§4.4).
-- `$__timeFilter` binds the range to `ts`; `$limit` caps the result set.
SELECT site_id,
       SUM(value) AS usage
FROM   histories
WHERE  tenant_id = $caller_tenant_id
  AND  $__timeFilter(ts)
GROUP  BY site_id
ORDER  BY usage DESC
LIMIT  $limit;
