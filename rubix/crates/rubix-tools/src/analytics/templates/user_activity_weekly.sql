-- user_activity_weekly — distinct active actors per day over the
-- last 7 days. Reads the ClickHouse mirror of the changelog.
SELECT
    toStartOfDay(toDateTime(epoch_ms / 1000)) AS day,
    uniqExact(actor_id)                        AS active_users
FROM changelog
WHERE epoch_ms >= toUnixTimestamp(now() - INTERVAL 7 DAY) * 1000
GROUP BY day
ORDER BY day
