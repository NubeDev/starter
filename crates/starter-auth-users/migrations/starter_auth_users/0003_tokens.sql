-- starter-auth-users API tokens (machine-client credentials).
-- Token format: `sak_<public_id>.<secret>` — the cleartext shown to
-- the user once, then split: `public_id` is what we look up (O(1)),
-- `secret` is what we argon2-hash and compare.
-- Scopes are stored as a JSON-encoded array of strings.
CREATE TABLE IF NOT EXISTS starter_auth_users_tokens (
    id            TEXT PRIMARY KEY,
    user_id       TEXT NOT NULL REFERENCES starter_auth_users_users(id) ON DELETE CASCADE,
    hashed_token  TEXT NOT NULL,
    scopes        TEXT NOT NULL DEFAULT '[]',
    created_at    TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at  TEXT,
    expires_at    TEXT,
    revoked_at    TEXT
);

CREATE INDEX IF NOT EXISTS starter_auth_users_tokens_user_id_idx
    ON starter_auth_users_tokens(user_id);
