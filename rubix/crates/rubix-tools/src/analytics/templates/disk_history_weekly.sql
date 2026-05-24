-- disk_history_weekly — per-day average and peak disk usage over
-- the last 7 days. Reads the L1 `system_disk_history` table that
-- `rubix.system.disk` writes to (see rubix-agent migration
-- 0002_history). The window is closed at "now - 7 days"; rows
-- partition by month so the scan stays cheap.
SELECT
    toStartOfDay(toDateTime(epoch_ms / 1000)) AS day,
    avg(percent_used)                          AS avg_percent,
    max(percent_used)                          AS peak_percent
FROM system_disk_history
WHERE epoch_ms >= toUnixTimestamp(now() - INTERVAL 7 DAY) * 1000
GROUP BY day
ORDER BY day
