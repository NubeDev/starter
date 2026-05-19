-- starter-auth-oauth migration 0003 (postgres flavour). See the
-- sqlite copy for the column-by-column rationale; this file diverges
-- only in `created_at` (`TIMESTAMPTZ` instead of `TEXT`).
CREATE TABLE IF NOT EXISTS starter_auth_oauth_state (
    state             TEXT        NOT NULL PRIMARY KEY,
    provider          TEXT        NOT NULL,
    pkce_verifier     TEXT        NOT NULL,
    return_to         TEXT,
    link_mode_user_id TEXT,
    created_at        TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_oauth_state_created_at
    ON starter_auth_oauth_state(created_at);
