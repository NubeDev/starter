-- starter-changelog Postgres backend — initial schema.
-- See `DOCS/backend/undo-redo/SCOPE.md` §"Storage shape".
--
-- `actor_model` is a generated column derived from
-- `actor_meta->>'model'`, kept indexed so the agent-log can filter
-- by model without a JSON extract per row. The SQLite backend
-- writes the same column explicitly — both shapes converge on the
-- same logical schema.

CREATE TABLE IF NOT EXISTS starter_changes (
    id               TEXT PRIMARY KEY,
    at               TIMESTAMPTZ NOT NULL,
    actor_kind       TEXT NOT NULL,
    actor_id         TEXT,
    actor_meta       JSONB,
    actor_model      TEXT GENERATED ALWAYS AS (actor_meta->>'model') STORED,
    resource_kind    TEXT NOT NULL,
    resource_id      TEXT NOT NULL,
    resource_owner   TEXT,
    resource_version BIGINT,
    op               TEXT NOT NULL,
    before           JSONB,
    after            JSONB,
    patch            JSONB,
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
