-- meter_litres_last_24h — sum of `value_avg` per water meter
-- over the last 24h, read from the L3 15-minute mart. Stage 05
-- dashboard KPI source: "Site A — last 24h L".
SELECT
    meter_id,
    sum(value_avg) AS litres
FROM rubix.meter_readings_15m
WHERE tenant_id = {tenant_id:String}
  AND kind = 'water'
  AND bucket_start >= now() - INTERVAL 24 HOUR
GROUP BY meter_id
ORDER BY meter_id
