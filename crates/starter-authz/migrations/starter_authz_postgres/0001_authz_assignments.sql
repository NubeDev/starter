-- Authz role assignments. See sqlite variant for column docs.
CREATE TABLE IF NOT EXISTS starter_authz_assignments (
    id          TEXT PRIMARY KEY,
    subject     TEXT NOT NULL,
    role        TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by  TEXT NOT NULL,
    UNIQUE (subject, role)
);

CREATE INDEX IF NOT EXISTS idx_authz_assignments_subject
    ON starter_authz_assignments (subject);
