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
--  3. `earliest` / `latest` are a bare `min/max("timestamp")` with NO
--     tenant predicate, so TimescaleDB reads per-chunk min/max metadata
--     (O(chunks), milliseconds) instead of scanning rows.
--
--     WHY NO `WHERE tenant_id = …` HERE (this was an 18s regression):
--     the host compiles `$caller_tenant_id` for a super-admin (`'*'`)
--     into `tenant_id = (CASE WHEN $1='*' THEN tenant_id ELSE $1 END)`,
--     i.e. `tenant_id = tenant_id` — a runtime-evaluated predicate the
--     planner can NOT constant-fold, which defeats chunk exclusion on
--     `"timestamp"` and forces a full ~955M-row scan (×2, min and max ≈
--     18s). Any `tenant_id` filter on the hypertable defeats the
--     timestamp chunk-exclusion fast path unless a matching composite
--     index exists. The earliest/latest *sample time* is a global
--     property of the data range; per-tenant scoping that actually
--     matters for the KPI is already applied to `point_count` via the
--     small `points` dimension table. On the adopted single-tenant
--     ('system') DB the two are identical anyway.
SELECT approximate_row_count('com_nubeio_rubixos__histories')::bigint AS sample_count,
       (SELECT count(*)::bigint
          FROM com_nubeio_rubixos__points
         WHERE tenant_id = $caller_tenant_id)                         AS point_count,
       (SELECT min("timestamp")
          FROM com_nubeio_rubixos__histories)                         AS earliest,
       (SELECT max("timestamp")
          FROM com_nubeio_rubixos__histories)                         AS latest;
