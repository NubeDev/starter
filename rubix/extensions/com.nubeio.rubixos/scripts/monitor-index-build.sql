-- monitor-index-build.sql
--
-- Read-only progress view for the long-running background build of
-- `com_nubeio_rubixos__idx_histories` on the 955M-row hypertable.
-- Touches no data. Run it repeatedly (e.g. `watch`-style) to follow a
-- build that the agent kicked off on boot.
--
--   export RUBIX_PROBE_DSN='postgres://postgres:<urlenc-pwd>@host:5432/postgres'
--   cargo run -p rubix-agent --example pg_probe -- \
--     "$(cat rubix/extensions/com.nubeio.rubixos/scripts/monitor-index-build.sql)"
--
-- Three things to look at:
--   1. Is the index present yet?  (build done once it appears)
--   2. Live CREATE INDEX backends — leader + parallel workers, runtime.
--   3. pg_stat_progress_create_index — blocks done / total + phase.

-- 1. Has the index committed? Non-empty => build is complete.
SELECT indexname
FROM pg_indexes
WHERE schemaname = 'public'
  AND tablename = 'com_nubeio_rubixos__histories'
  AND indexname = 'com_nubeio_rubixos__idx_histories';

-- 2. Live CREATE INDEX backends for this index (leader + workers).
SELECT pid,
       leader_pid,
       state,
       wait_event_type,
       wait_event,
       EXTRACT(EPOCH FROM (now() - query_start))::int AS runtime_s
FROM pg_stat_activity
WHERE state = 'active'
  AND query ILIKE '%CREATE INDEX%com_nubeio_rubixos__idx_histories%'
ORDER BY leader_pid NULLS FIRST, pid;

-- 3. Native build progress (Postgres 12+). `phase` walks through
-- "building index", "loading tuples in tree", etc.; the blocks_*
-- columns give a rough percent on the scan phase.
SELECT p.pid,
       p.phase,
       p.blocks_done,
       p.blocks_total,
       CASE WHEN p.blocks_total > 0
            THEN round(100.0 * p.blocks_done / p.blocks_total, 1)
            ELSE NULL END AS pct,
       p.tuples_done,
       p.partitions_done,
       p.partitions_total
FROM pg_stat_progress_create_index p
JOIN pg_class c ON c.oid = p.relid
WHERE c.relname LIKE 'com_nubeio_rubixos__histories%';
