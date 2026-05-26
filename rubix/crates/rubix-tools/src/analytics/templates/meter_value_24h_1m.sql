-- meter_value_24h_1m — 1-minute granularity series for a single
-- meter over the last 24h, read from the L2 mart. Stage 05 success
-- bar item 4: when a chart zooms to ≤ 6h the SDUI resolver swaps
-- the L3 15-min template for this one so the user sees the finer
-- grain. The bundled `data-flow-site-a` dashboard pairs this
-- template with `meter_value_30d_15m` in a second row so the
-- cross-over is observable today without interactive zoom support
-- on the chart kind (see the stage-05 follow-up note).
-- L2 stores `value Nullable(Float64)`; the resolver maps it to the
-- chart's `value_field` via the AnalyticsTemplateMap so dashboards
-- can keep a single mapping (`value_avg`) by aliasing here.
SELECT
    meter_id,
    bucket_start,
    value AS value_avg
FROM rubix.meter_readings_1m
WHERE tenant_id = {tenant_id:String}
  AND meter_id  = {meter_id:String}
  AND bucket_start >= now() - INTERVAL 24 HOUR
  AND value IS NOT NULL
ORDER BY bucket_start
