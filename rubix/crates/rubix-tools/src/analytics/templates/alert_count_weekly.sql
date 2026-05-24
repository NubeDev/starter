-- alert_count_weekly — number of `rubix.alert.send` invocations per
-- severity in the last 7 days. Reads the ClickHouse mirror of the
-- changelog (table `changelog`) that the recorder writes alongside
-- the PG row; the column shapes follow `starter_spi::changelog::Change`.
SELECT
    severity,
    count() AS n
FROM changelog
WHERE verb = 'rubix.alert.send'
  AND epoch_ms >= toUnixTimestamp(now() - INTERVAL 7 DAY) * 1000
GROUP BY severity
ORDER BY n DESC
