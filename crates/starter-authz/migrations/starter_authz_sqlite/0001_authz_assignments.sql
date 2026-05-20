-- Authz role assignments. Subject is an exact id ("alice@example.com")
-- or a single-trailing-* glob ("*@acme.com"); role is a free-form
-- name matching `Rule::role`.
CREATE TABLE IF NOT EXISTS starter_authz_assignments (
    id          TEXT PRIMARY KEY,
    subject     TEXT NOT NULL,
    role        TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by  TEXT NOT NULL,
    UNIQUE (subject, role)
);

CREATE INDEX IF NOT EXISTS idx_authz_assignments_subject
    ON starter_authz_assignments (subject);
