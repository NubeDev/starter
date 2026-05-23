-- starter-auth-users API tokens (machine-client credentials).
-- Postgres mirror of 0003_tokens.sql in migrations/starter_auth_users/.
--
-- Translation notes:
--   sqlite TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
--   →     TIMESTAMPTZ NOT NULL DEFAULT NOW()
--   sqlite TEXT (rfc3339) for nullable timestamps
--   →     TIMESTAMPTZ
--   sqlite TEXT NOT NULL DEFAULT '[]' for the scopes JSON array
--   →     JSONB  NOT NULL DEFAULT '[]'::jsonb
--          (Postgres has a real JSON type — store + index efficiently;
--          the application still treats the column as JSON-encoded.)
--
-- Token format: `sak_<public_id>.<secret>` — the cleartext shown to
-- the user once, then split: `public_id` is what we look up (O(1)),
-- `secret` is what we argon2-hash and compare.
CREATE TABLE IF NOT EXISTS starter_auth_users_tokens (
    id            TEXT PRIMARY KEY,
    user_id       TEXT NOT NULL REFERENCES starter_auth_users_users(id) ON DELETE CASCADE,
    hashed_token  TEXT NOT NULL,
    scopes        JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at  TIMESTAMPTZ,
    expires_at    TIMESTAMPTZ,
    revoked_at    TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS starter_auth_users_tokens_user_id_idx
    ON starter_auth_users_tokens(user_id);
