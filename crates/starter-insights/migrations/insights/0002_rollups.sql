-- Insights Phase 2 schema: verdict rollups (tier 2 materialisation).
--
-- Per DOCS/Insights/SCOPE.md Materialisation:
-- - `verdict_rollup` aggregates the verdict log on a scheduled
--   trigger. Incremental by default: each tick reads only verdicts
--   since the per-`(rule_id, window_class)` checkpoint and merges
--   into the existing aggregate.
-- - `rollup_invalidation` is the D5 per-window watermark seam.
--   Retroactive corrections enqueue `(rule_id, window_start,
--   window_end, reason)` rows here; the scheduled rollup job drains
--   the queue.
-- - Tag-grouped rollups (R-ins-8) live in the same table, with
--   `tag_key` / `tag_value` columns NULLable so an "ungrouped" row
--   coexists with per-tag aggregates.

CREATE TABLE IF NOT EXISTS verdict_rollup (
    rule_namespace TEXT    NOT NULL,
    rule_name      TEXT    NOT NULL,
    rule_major     INTEGER NOT NULL,
    window_class   TEXT    NOT NULL,           -- 'hour' | 'day' | 'week'
    window_start_ms INTEGER NOT NULL,
    window_end_ms   INTEGER NOT NULL,
    tag_key        TEXT,                       -- NULL = ungrouped
    tag_value      TEXT,
    count_healthy  INTEGER NOT NULL DEFAULT 0,
    count_info     INTEGER NOT NULL DEFAULT 0,
    count_warn     INTEGER NOT NULL DEFAULT 0,
    count_critical INTEGER NOT NULL DEFAULT 0,
    count_error    INTEGER NOT NULL DEFAULT 0,
    coverage_min   REAL,                       -- min effective.confidence
    stale_since_ms INTEGER                     -- D5 retroactive marker
);

-- Composite uniqueness via an explicit expression index (SQLite
-- forbids expressions in PRIMARY KEY / UNIQUE table constraints).
CREATE UNIQUE INDEX IF NOT EXISTS uq_verdict_rollup
    ON verdict_rollup (rule_namespace, rule_name, rule_major,
                       window_class, window_start_ms,
                       IFNULL(tag_key, ''), IFNULL(tag_value, ''));

CREATE INDEX IF NOT EXISTS idx_verdict_rollup_rule_window
    ON verdict_rollup (rule_namespace, rule_name, rule_major,
                       window_class, window_start_ms);

CREATE INDEX IF NOT EXISTS idx_verdict_rollup_tag
    ON verdict_rollup (tag_key, tag_value);

-- Per-(rule_id, window_class) checkpoint of the most-recent at_ms
-- that has been folded into the rollup. Phase 2 default — non-
-- retroactive rules. D5 retroactive rules read the watermark from
-- `rollup_invalidation` for the affected windows.
CREATE TABLE IF NOT EXISTS rollup_checkpoint (
    rule_namespace TEXT    NOT NULL,
    rule_name      TEXT    NOT NULL,
    rule_major     INTEGER NOT NULL,
    window_class   TEXT    NOT NULL,
    last_at_ms     INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (rule_namespace, rule_name, rule_major, window_class)
);

-- D5 per-window invalidation queue. Retroactive rules push rows
-- here on input mutation; the scheduled rollup job drains, recomputes,
-- and clears `stale_since_ms` on the rollup row.
CREATE TABLE IF NOT EXISTS rollup_invalidation (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    rule_namespace  TEXT    NOT NULL,
    rule_name       TEXT    NOT NULL,
    rule_major      INTEGER NOT NULL,
    window_class    TEXT    NOT NULL,
    window_start_ms INTEGER NOT NULL,
    window_end_ms   INTEGER NOT NULL,
    reason          TEXT    NOT NULL,
    enqueued_ms     INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_rollup_invalidation_rule
    ON rollup_invalidation (rule_namespace, rule_name, rule_major,
                            window_class, window_start_ms);
