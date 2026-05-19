-- Postgres twin of the sqlite migration. Same single-row guard,
-- same column set; `BYTEA` for bytes, `JSONB` for the JSON columns
-- so range queries (if a consumer ever adds them) stay efficient.
CREATE TABLE IF NOT EXISTS starter_ui_theme (
    id              SMALLINT PRIMARY KEY CHECK (id = 1),
    theme_styles    JSONB NOT NULL DEFAULT '{"light":{},"dark":{}}'::jsonb,
    shell           JSONB NOT NULL DEFAULT '{"nav_title":"","hide_features":[]}'::jsonb,
    logo_bytes      BYTEA,
    logo_mime       TEXT,
    favicon_bytes   BYTEA,
    favicon_mime    TEXT,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
