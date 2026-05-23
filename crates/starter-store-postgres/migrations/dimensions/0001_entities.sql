-- Warehouse SCOPE W1, L2-dim: canonical entity dimension table.
-- Tags are JSONB; the GIN jsonb_path_ops index accelerates the
-- containment queries produced by `starter_tags::compile_to_pg`.

CREATE TABLE IF NOT EXISTS entities (
    id          TEXT PRIMARY KEY,
    kind        TEXT NOT NULL,
    display     TEXT,
    tags        JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS entities_tags_gin
    ON entities USING GIN (tags jsonb_path_ops);
CREATE INDEX IF NOT EXISTS entities_kind
    ON entities (kind);
