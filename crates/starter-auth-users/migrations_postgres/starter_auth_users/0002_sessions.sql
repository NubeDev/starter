-- starter-auth-users session records. Postgres mirror of
-- 0002_sessions.sql in migrations/starter_auth_users/.
--
-- Translation notes:
--   sqlite TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
--   →     TIMESTAMPTZ NOT NULL DEFAULT NOW()
--   sqlite TEXT (storing rfc3339)  for nullable timestamps
--   →     TIMESTAMPTZ
--
-- `tenant_id` is the session-to-tenant binding written at login
-- from the user's membership row. Sqlite picks the column up in
-- a later migration (`0005_tenants.sql ALTER TABLE`); the Postgres
-- migration set is fresh, so the long-term-correct shape carries
-- the column in 0002 directly, matching the final sqlite schema.
-- See docs/design/auth/README.md for the multi-tenant session
-- contract.
--
-- The opaque session id stored in the `starter_session` cookie is
-- `id` directly. Sessions are short-lived; expired rows are reaped
-- lazily on read. Revoked rows carry `revoked_at` and are treated
-- as expired by verify.
CREATE TABLE IF NOT EXISTS starter_auth_users_sessions (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES starter_auth_users_users(id) ON DELETE CASCADE,
    csrf_token  TEXT NOT NULL,
    tenant_id   TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at  TIMESTAMPTZ NOT NULL,
    revoked_at  TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS starter_auth_users_sessions_user_id_idx
    ON starter_auth_users_sessions(user_id);

CREATE INDEX IF NOT EXISTS starter_auth_users_sessions_tenant_id_idx
    ON starter_auth_users_sessions(tenant_id);
