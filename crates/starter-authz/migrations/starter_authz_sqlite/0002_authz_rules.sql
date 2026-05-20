-- Authz rules. Evaluated in (priority DESC, id ASC) order; deny
-- always wins on conflict (SCOPE.md R3, deny-overrides).
--
-- `actions` is a JSON array of action strings. `["*"]` matches any
-- action; otherwise membership is exact. `condition` is either NULL,
-- the magic keyword "owner" (`principal.subject == object.owner`),
-- or an expression in the condition mini-language (SCOPE.md R8).
CREATE TABLE IF NOT EXISTS starter_authz_rules (
    id          TEXT PRIMARY KEY,
    role        TEXT NOT NULL,
    resource    TEXT NOT NULL,
    actions     TEXT NOT NULL,
    condition   TEXT,
    effect      TEXT NOT NULL CHECK (effect IN ('allow', 'deny')),
    priority    INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_authz_rules_role_resource
    ON starter_authz_rules (role, resource);
