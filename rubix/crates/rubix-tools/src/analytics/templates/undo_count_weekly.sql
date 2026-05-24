-- undo_count_weekly — number of `rubix.undo.last` invocations per
-- day over the last 7 days. A spike here usually means an operator
-- (or LLM) is fighting a misbehaving tool — surface it in the
-- weekly report.
SELECT
    toStartOfDay(toDateTime(epoch_ms / 1000)) AS day,
    count()                                    AS undos
FROM changelog
WHERE verb = 'rubix.undo.last'
  AND epoch_ms >= toUnixTimestamp(now() - INTERVAL 7 DAY) * 1000
GROUP BY day
ORDER BY day
