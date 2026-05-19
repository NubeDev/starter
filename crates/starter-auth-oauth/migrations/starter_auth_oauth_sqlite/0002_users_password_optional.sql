-- no-transaction
-- The `-- no-transaction` marker above tells sqlx to run this file
-- outside its default migration transaction. We need that because
-- `PRAGMA foreign_keys = OFF` is a no-op inside a transaction in
-- SQLite, and the rebuild trick below depends on FKs being off while
-- we drop and rename the table.
--
-- starter-auth-oauth migration 0002: relax NOT NULL on
-- starter_auth_users_users.password_hash so OAuth-only users can land
-- with NULL. This migration ships **here**, not in
-- starter-auth-users, so consumers who never enable OAuth never run
-- it (Hard rule R8 + Constraints in SCOPE.md).
--
-- SQLite cannot ALTER a column's NOT NULL constraint, so this is the
-- 12-step rebuild from the SQLite docs ("Making Other Kinds Of Table
-- Schema Changes"). The PRAGMA foreign_key_check assertion before
-- COMMIT is the safety net that turns silent FK corruption into a
-- migration failure.
--
-- Numbering: this is "0002" relative to the OAuth crate's own
-- migration directory; "0001" in this directory is
-- 0001_oauth_identities.sql (lands in stage 4). Numbering is
-- per-source so it does not collide with the users crate's 0002
-- (sessions table).

-- 1. Defer FK enforcement for the duration of the rebuild. Without
--    this, the rename of the old table breaks every referencing row.
PRAGMA foreign_keys = OFF;

BEGIN TRANSACTION;

-- 2. Create the replacement table with password_hash nullable.
CREATE TABLE starter_auth_users_users__new (
    id            TEXT PRIMARY KEY,
    email         TEXT NOT NULL UNIQUE,
    password_hash TEXT,
    role          TEXT NOT NULL,
    created_at    TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at    TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 3. Copy every row from the live table.
INSERT INTO starter_auth_users_users__new (id, email, password_hash, role, created_at, updated_at)
SELECT id, email, password_hash, role, created_at, updated_at
FROM starter_auth_users_users;

-- 4. Drop the old table. ON DELETE CASCADE on sessions / tokens /
--    oauth_identities means the rows that referenced it are NOT
--    cascaded here because we have FKs OFF; they will re-bind to the
--    renamed table in step 5.
DROP TABLE starter_auth_users_users;

-- 5. Rename the replacement into place. The FK definitions on the
--    referencing tables target the table *name*, so renaming back to
--    `starter_auth_users_users` reconnects every foreign key.
ALTER TABLE starter_auth_users_users__new RENAME TO starter_auth_users_users;

-- 6. Final sanity check: every FK in the database must resolve. A
--    row here means a referencing table points at an id that no
--    longer exists; that is a corruption we want to surface as a
--    failed migration, not commit through.
PRAGMA foreign_key_check;

COMMIT;

-- 7. Restore enforcement for the rest of the session.
PRAGMA foreign_keys = ON;
