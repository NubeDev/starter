-- Consumer-owned migration source. Applied through starter's
-- namespaced migration runner alongside `starter_auth_token`'s own
-- migrations — each lands in its own `_sqlx_migrations_<name>` table.

CREATE TABLE IF NOT EXISTS notes (
    id          TEXT PRIMARY KEY,
    body        TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    created_by  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS notes_created_at_idx ON notes (created_at DESC);
