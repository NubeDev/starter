-- clickhouse_writes_weekly — count of mutating clickhouse.* verbs
-- in the last 7 days, grouped by verb. Reads the ClickHouse mirror
-- of the changelog. Useful to spot operators DDL-thrashing the
-- warehouse.
SELECT
    verb,
    count() AS n
FROM changelog
WHERE verb LIKE 'rubix.clickhouse.%'
  AND epoch_ms >= toUnixTimestamp(now() - INTERVAL 7 DAY) * 1000
GROUP BY verb
ORDER BY n DESC
