-- com.nubeio.rubixos.history_recent
--
-- Newest `limit` raw samples for a single point. The
-- (tenant_id, timestamp, point_uuid) index lives on the host-
-- created hypertable, so this is an index scan even on the full
-- 17M-row table.
SELECT "timestamp",
       value,
       host_uuid
FROM   com_nubeio_rubixos__histories
WHERE  tenant_id  = $caller_tenant_id
  AND  point_uuid = $point_uuid
ORDER  BY "timestamp" DESC
LIMIT  $limit;
