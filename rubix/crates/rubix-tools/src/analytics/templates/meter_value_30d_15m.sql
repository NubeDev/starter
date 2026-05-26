-- meter_value_30d_15m — 15-minute average value for one meter
-- over the last 30 days, read from the L3 mart. Stage 05 dashboard
-- chart source: one row per bucket, ordered oldest-first so the
-- SDUI chart resolver can stitch (bucket_start, value_avg) into a
-- single line series.
SELECT
    meter_id,
    bucket_start,
    value_avg
FROM rubix.meter_readings_15m
WHERE tenant_id = {tenant_id:String}
  AND meter_id  = {meter_id:String}
  AND bucket_start >= now() - INTERVAL 30 DAY
ORDER BY bucket_start
