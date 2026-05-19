-- Single-row table holding the org-level theme. The
-- `CHECK (id = 1)` is the entire multi-row guard — every backend
-- impl writes id = 1 and reads id = 1, so no transaction is needed
-- to ensure singleton semantics.
CREATE TABLE IF NOT EXISTS starter_ui_theme (
    id              INTEGER PRIMARY KEY CHECK (id = 1),
    theme_styles    TEXT NOT NULL DEFAULT '{"light":{},"dark":{}}',
    shell           TEXT NOT NULL DEFAULT '{"nav_title":"","hide_features":[]}',
    logo_bytes      BLOB,
    logo_mime       TEXT,
    favicon_bytes   BLOB,
    favicon_mime    TEXT,
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
