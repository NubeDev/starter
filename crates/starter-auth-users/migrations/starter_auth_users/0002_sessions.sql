-- starter-auth-users session records. The opaque session id stored
-- in the `starter_session` cookie is `id` directly. Sessions are
-- short-lived; expired rows are reaped lazily on read. Revoked rows
-- carry `revoked_at` and are treated as expired by verify.
CREATE TABLE IF NOT EXISTS starter_auth_users_sessions (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES starter_auth_users_users(id) ON DELETE CASCADE,
    csrf_token  TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at  TEXT NOT NULL,
    revoked_at  TEXT
);

CREATE INDEX IF NOT EXISTS starter_auth_users_sessions_user_id_idx
    ON starter_auth_users_sessions(user_id);
