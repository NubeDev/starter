-- Resolve SCOPE open question #3: align Postgres with SQLite by
-- having the recorder write `actor_model` explicitly instead of
-- deriving it server-side. The recorder now owns the contract for
-- every column.
--
-- This migration:
--   1. Drops the GENERATED column added in 0001.
--   2. Re-adds `actor_model` as a regular nullable TEXT column.
--   3. Backfills existing rows from `actor_meta->>'model'`.
--   4. Recreates the `(actor_kind, actor_model, at DESC)` index that
--      the agent-log relies on for filter-by-model queries.

ALTER TABLE starter_changes DROP COLUMN actor_model;

ALTER TABLE starter_changes ADD COLUMN actor_model TEXT;

UPDATE starter_changes
   SET actor_model = actor_meta->>'model'
 WHERE actor_meta IS NOT NULL
   AND actor_meta ? 'model';

CREATE INDEX IF NOT EXISTS idx_starter_changes_actor_model
    ON starter_changes (actor_kind, actor_model, at DESC);
