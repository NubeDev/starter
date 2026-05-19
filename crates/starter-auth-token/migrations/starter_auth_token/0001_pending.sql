-- starter-auth-token pending row. Holds the plaintext claim token
-- until the operator consumes it via POST /auth/claim. At most one
-- row exists at any time; the runtime enforces the cap via DELETE-
-- then-INSERT in `reset_with_new_pending`.
CREATE TABLE IF NOT EXISTS starter_auth_token_pending (
    id          TEXT PRIMARY KEY,
    plaintext   TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Track the auth epoch separately so factory-reset can bump it
-- atomically with the pending-row rewrite.
CREATE TABLE IF NOT EXISTS starter_auth_token_epoch (
    id    INTEGER PRIMARY KEY CHECK (id = 1),
    epoch INTEGER NOT NULL DEFAULT 0
);
INSERT OR IGNORE INTO starter_auth_token_epoch (id, epoch) VALUES (1, 0);
