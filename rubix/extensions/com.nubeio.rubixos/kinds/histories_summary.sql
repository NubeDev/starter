-- com.nubeio.rubixos.histories_summary
--
-- One-row KPI: row count + distinct-point count + min/max timestamp
-- for the dashboard header cards.
--
-- Performance: the previous version did `count(*)`, `count(DISTINCT
-- point_uuid)`, and `min/max("timestamp")` directly on the histories
-- hypertable. On a populated dump that was a full sequential scan and
-- took ~6s per call. Three rewrites:
--
--  1. `sample_count` uses Timescale's `approximate_row_count()` which
--     reads chunk metadata only (microseconds, not seconds). Slightly
--     loose under heavy ingest but fine for a header card.
--  2. `point_count` comes from the small `points` dimension table, not
--     a DISTINCT scan over every history row.
--  3. `earliest` / `latest` stay as `min/max("timestamp")` — Timescale's
--     chunk exclusion makes those O(chunks), not O(rows).
SELECT approximate_row_count('com_nubeio_rubixos__histories')::bigint AS sample_count,
       (SELECT count(*)::bigint
          FROM com_nubeio_rubixos__points
         WHERE tenant_id = $caller_tenant_id)                         AS point_count,
       (SELECT min("timestamp")
          FROM com_nubeio_rubixos__histories
         WHERE tenant_id = $caller_tenant_id)                         AS earliest,
       (SELECT max("timestamp")
          FROM com_nubeio_rubixos__histories
         WHERE tenant_id = $caller_tenant_id)                         AS latest;
