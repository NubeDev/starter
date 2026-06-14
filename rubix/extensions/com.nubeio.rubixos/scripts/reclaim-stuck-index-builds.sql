-- reclaim-stuck-index-builds.sql
--
-- Operational, DATA-SAFE cleanup for the adopted Rubix-OS Timescale DB.
-- Touches NO table data — it only cancels runaway *server-side* backends
-- that a previous agent left behind on the shared host:
--
--   * duplicate `CREATE INDEX … com_nubeio_rubixos__idx_histories` builds
--     stacked up by repeated agent boots (before the single-flight
--     advisory lock landed — see boot/extension_tables.rs), and
--   * orphaned `count(*)` probes over the 955M-row hypertable whose
--     client died but whose backend kept running.
--
-- It KEEPS the one genuinely-progressing index build (the oldest leader
-- backend and its parallel workers) so an in-flight multi-hour build is
-- not thrown away, and cancels only the redundant ones.
--
-- `pg_cancel_backend` (SIGINT) is used, not `pg_terminate_backend`
-- (SIGTERM): cancelling rolls back the statement cleanly. A cancelled
-- `CREATE INDEX IF NOT EXISTS` leaves no partial index behind.
--
-- Idempotent and re-runnable: with nothing stuck it cancels nothing and
-- returns 0 rows. Read-only against all user tables.

WITH index_builds AS (
    -- One row per *leader* CREATE INDEX backend on histories (exclude
    -- parallel workers — they share the leader's transaction and die
    -- with it). `leader_pid = pid` for a non-parallel backend.
    SELECT pid, query_start
    FROM pg_stat_activity
    WHERE state = 'active'
      AND pid <> pg_backend_pid()
      AND (leader_pid IS NULL OR leader_pid = pid)
      AND query ILIKE '%CREATE INDEX%com_nubeio_rubixos__idx_histories%'
),
keep AS (
    -- Keep the oldest build (most progress); it will finish and create
    -- the index. Everything else is a redundant duplicate.
    SELECT pid FROM index_builds ORDER BY query_start ASC LIMIT 1
),
duplicate_builds AS (
    SELECT pid FROM index_builds WHERE pid NOT IN (SELECT pid FROM keep)
),
orphan_counts AS (
    -- Stray full-table counts over the giant hypertable — never part of
    -- normal serving; safe to cancel.
    SELECT pid
    FROM pg_stat_activity
    WHERE state = 'active'
      AND pid <> pg_backend_pid()
      AND (leader_pid IS NULL OR leader_pid = pid)
      AND query ILIKE '%count(*)%com_nubeio_rubixos__histories%'
),
to_cancel AS (
    SELECT pid, 'duplicate index build' AS reason FROM duplicate_builds
    UNION ALL
    SELECT pid, 'orphaned count(*)'      AS reason FROM orphan_counts
)
SELECT
    c.pid,
    c.reason,
    pg_cancel_backend(c.pid) AS cancel_signalled,
    EXTRACT(EPOCH FROM (now() - a.query_start))::int AS runtime_s,
    left(regexp_replace(a.query, '\s+', ' ', 'g'), 80) AS query
FROM to_cancel c
JOIN pg_stat_activity a ON a.pid = c.pid
ORDER BY c.reason, c.pid;
