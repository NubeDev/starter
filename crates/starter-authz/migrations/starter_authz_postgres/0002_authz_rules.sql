-- Authz rules. See sqlite variant for column docs. `actions` is
-- JSONB on postgres to match the workspace convention.
CREATE TABLE IF NOT EXISTS starter_authz_rules (
    id          TEXT PRIMARY KEY,
    role        TEXT NOT NULL,
    resource    TEXT NOT NULL,
    actions     JSONB NOT NULL,
    condition   TEXT,
    effect      TEXT NOT NULL CHECK (effect IN ('allow', 'deny')),
    priority    INTEGER NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_authz_rules_role_resource
    ON starter_authz_rules (role, resource);
