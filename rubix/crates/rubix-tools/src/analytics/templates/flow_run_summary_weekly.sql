-- flow_run_summary_weekly — flow runs grouped by terminal status
-- over the last 7 days. Reads the `flow_run_history` mart written
-- by the flow audit sink.
SELECT
    status,
    count() AS runs
FROM flow_run_history
WHERE epoch_ms >= toUnixTimestamp(now() - INTERVAL 7 DAY) * 1000
GROUP BY status
ORDER BY runs DESC
