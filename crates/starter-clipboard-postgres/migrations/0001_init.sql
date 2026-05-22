-- starter-clipboard Postgres backend — initial schema.
--
-- See `DOCS/backend/undo-redo/SCOPE.md` §"Storage shape".
-- HMAC signature is computed over
-- `principal_id | "\x1e" | resource_kind | "\x1e" | payload_canonical`
-- with the key fetched from `starter_spi::SecretStore` under
-- `starter.clipboard.hmac`.

CREATE TABLE IF NOT EXISTS starter_clipboard (
    id            TEXT        PRIMARY KEY,
    principal_id  TEXT        NOT NULL,
    resource_kind TEXT        NOT NULL,
    payload       TEXT        NOT NULL,
    signature     BYTEA       NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL,
    expires_at    TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_starter_clipboard_principal
    ON starter_clipboard (principal_id, expires_at);
