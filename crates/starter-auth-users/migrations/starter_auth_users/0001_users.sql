-- starter-auth-users user records. One row per user; email is the
-- primary external identifier. Password is stored as an argon2id PHC
-- string ($argon2id$v=19$...). Role is one of reader/writer/admin.
CREATE TABLE IF NOT EXISTS starter_auth_users_users (
    id            TEXT PRIMARY KEY,
    email         TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role          TEXT NOT NULL,
    created_at    TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at    TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
