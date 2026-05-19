-- starter-auth-token claimed row. Holds the SHA-256 digest of the
-- issued owner token. At most one row exists at any time; promotion
-- from pending is done in a single transaction with the pending
-- DELETE.
CREATE TABLE IF NOT EXISTS starter_auth_token_claimed (
    claim_id    TEXT PRIMARY KEY,
    digest      BLOB NOT NULL,
    claimed_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
