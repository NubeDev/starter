-- com.nubeio.rubixos.histories_summary
--
-- One-row KPI: row count + distinct-point count + min/max timestamp
-- for the dashboard header cards.
SELECT count(*)::bigint            AS sample_count,
       count(DISTINCT point_uuid)  AS point_count,
       min("timestamp")            AS earliest,
       max("timestamp")            AS latest
FROM   com_nubeio_rubixos__histories
WHERE  tenant_id = $caller_tenant_id;
