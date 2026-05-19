-- starter-auth-oauth migration 0001 (postgres flavour). See the
-- sqlite copy of this file for the column-by-column rationale; this
-- file diverges only in types (TIMESTAMPTZ + NOW() instead of TEXT
-- + CURRENT_TIMESTAMP) and ships the same shape.
CREATE TABLE IF NOT EXISTS starter_auth_oauth_identities (
    provider      TEXT        NOT NULL,
    provider_sub  TEXT        NOT NULL,
    user_id       TEXT        NOT NULL,
    email         TEXT,
    display_name  TEXT,
    linked_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (provider, provider_sub),
    FOREIGN KEY (user_id) REFERENCES starter_auth_users_users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_oauth_identities_user
    ON starter_auth_oauth_identities(user_id);
