-- starter-changelog SQLite backend — initial schema.
--
-- See `DOCS/backend/undo-redo/SCOPE.md` §"Storage shape". The
-- `payload` columns (`before`, `after`, `patch`) are opaque JSON
-- TEXT — starter does not inspect them.
--
-- `id` is a ULID-shaped TEXT (uuid v7 in this backend), monotonic,
-- assigned by the recorder. `group_id` is assigned by
-- `ChangeRecorder::transaction` once and shared across every row
-- in the closure.
--
-- `actor_meta` carries kind-specific extras (e.g. `{"model": "..."}`
-- for agents). `actor_model` is denormalised so the agent-log can
-- index on `(actor_kind, actor_model, at DESC)` without a JSON
-- extract.

CREATE TABLE IF NOT EXISTS starter_changes (
    id               TEXT PRIMARY KEY,
    at               TEXT NOT NULL,
    actor_kind       TEXT NOT NULL,
    actor_id         TEXT,
    actor_meta       TEXT,
    actor_model      TEXT,
    resource_kind    TEXT NOT NULL,
    resource_id      TEXT NOT NULL,
    resource_owner   TEXT,
    resource_version INTEGER,
    op               TEXT NOT NULL,
    before           TEXT,
    after            TEXT,
    patch            TEXT,
    group_id         TEXT NOT NULL,
    correlation      TEXT
);

CREATE INDEX IF NOT EXISTS idx_starter_changes_resource
    ON starter_changes (resource_kind, resource_id, at DESC);

CREATE INDEX IF NOT EXISTS idx_starter_changes_actor
    ON starter_changes (actor_kind, actor_id, at DESC);

CREATE INDEX IF NOT EXISTS idx_starter_changes_actor_model
    ON starter_changes (actor_kind, actor_model, at DESC);

CREATE INDEX IF NOT EXISTS idx_starter_changes_group
    ON starter_changes (group_id);

CREATE INDEX IF NOT EXISTS idx_starter_changes_at
    ON starter_changes (at DESC, id DESC);
