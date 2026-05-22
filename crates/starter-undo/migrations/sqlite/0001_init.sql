-- starter-undo SQLite backend — initial schema.
--
-- Per-actor redo stack. SCOPE §"First concrete step" envisages a
-- small `starter_undo_cursors` table so undo survives process
-- restarts and works across server instances.
--
-- One row per stack entry. `position` is dense, monotonically
-- increasing per `actor_key`; the top of the stack is the row with
-- the largest `position`. The `(actor_key, position)` composite
-- primary key also serves as the natural lookup index.

CREATE TABLE IF NOT EXISTS starter_undo_cursors (
    actor_key  TEXT    NOT NULL,
    position   INTEGER NOT NULL,
    group_id   TEXT    NOT NULL,
    pushed_at  TEXT    NOT NULL,
    PRIMARY KEY (actor_key, position)
);
