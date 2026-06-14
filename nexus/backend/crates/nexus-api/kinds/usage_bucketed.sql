-- nexus.core.usage_bucketed
--
-- Time-bucketed energy usage over the dashboard time range, scoped to the
-- caller's tenant. The data side has no RLS, so the `$caller_tenant_id`
-- predicate is mandatory (§4.4) and host-bound — the caller cannot supply it.
-- `$__timeFilter` binds the range to `ts`; `$__timeGroup(ts, $__interval)`
-- buckets at the dashboard's resolution. An empty $site_id disables the site
-- filter; a non-empty one narrows to a single site.
SELECT $__timeGroup(ts, $__interval) AS bucket,
       site_id,
       SUM(value)                     AS usage
FROM   histories
WHERE  tenant_id = $caller_tenant_id
  AND  $__timeFilter(ts)
  AND  ($site_id = '' OR site_id = $site_id)
GROUP  BY bucket, site_id
ORDER  BY bucket, site_id;
