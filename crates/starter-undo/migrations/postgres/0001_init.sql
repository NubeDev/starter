-- starter-undo Postgres backend — initial schema.
--
-- Per-actor redo stack with an epoch column for optimistic
-- concurrency. The whole stack lives in a single JSONB array so
-- push/pop/clear are atomic CAS updates against `(actor_key, epoch)`:
-- two processes racing redo for the same actor cannot both pop the
-- same group, because the loser observes the bumped epoch and
-- retries (or surfaces the conflict).
--
-- Schema choices, with reasoning:
--
-- - `actor_key TEXT PRIMARY KEY` — `actor_key(actor)` from
--   `starter_undo::service` is already the canonical row key; one
--   row per actor (not one row per stack entry) keeps the hot
--   probes O(1) and lets the CAS-on-epoch idiom work without
--   touching multiple rows in a single transaction.
-- - `redo_stack JSONB NOT NULL DEFAULT '[]'::jsonb` — vector of
--   `GroupId` strings, top of the stack at the *end* of the array
--   so push/pop are O(1) JSONB ops.
-- - `epoch BIGINT NOT NULL DEFAULT 0` — monotonic, bumped on every
--   successful write. The CAS predicate is
--   `WHERE actor_key = $1 AND epoch = $observed`. A mismatched
--   epoch means another writer landed first; callers retry by
--   re-reading the row.
-- - `updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()` — used by the
--   TTL sweep that drops rows whose youngest referenced GroupId
--   has fallen outside undo retention. The sweep itself lives in
--   the rubix-agent boot crate (boot::undo_sweep) so it can read
--   the per-kind policy table alongside this one.

CREATE TABLE IF NOT EXISTS starter_undo_cursors (
    actor_key  TEXT        PRIMARY KEY,
    redo_stack JSONB       NOT NULL DEFAULT '[]'::jsonb,
    epoch      BIGINT      NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Cheap age scan for the retention sweep.
CREATE INDEX IF NOT EXISTS idx_starter_undo_cursors_updated_at
    ON starter_undo_cursors (updated_at);
