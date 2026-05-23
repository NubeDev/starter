-- starter-auth-users session records. Postgres mirror of
-- 0002_sessions.sql in migrations/starter_auth_users/.
--
-- Translation notes:
--   sqlite TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
--   →     TIMESTAMPTZ NOT NULL DEFAULT NOW()
--   sqlite TEXT (storing rfc3339)  for nullable timestamps
--   →     TIMESTAMPTZ
--
-- The opaque session id stored in the `starter_session` cookie is
-- `id` directly. Sessions are short-lived; expired rows are reaped
-- lazily on read. Revoked rows carry `revoked_at` and are treated
-- as expired by verify.
CREATE TABLE IF NOT EXISTS starter_auth_users_sessions (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES starter_auth_users_users(id) ON DELETE CASCADE,
    csrf_token  TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at  TIMESTAMPTZ NOT NULL,
    revoked_at  TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS starter_auth_users_sessions_user_id_idx
    ON starter_auth_users_sessions(user_id);
