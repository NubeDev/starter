-- meter_kwh_site_a_weekly — 7-day kWh sum per electricity meter for
-- tenant site-a, from the L3 15-minute mart. Self-contained (no CH
-- params) so rubix.analytics.report can run it with empty params.
SELECT
    meter_id,
    sum(value_avg) AS kwh,
    count()        AS bucket_count
FROM rubix.meter_readings_15m
WHERE tenant_id = 'site-a'
  AND kind = 'electricity'
  AND bucket_start >= now() - INTERVAL 7 DAY
GROUP BY meter_id
ORDER BY meter_id
