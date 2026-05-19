-- starter-auth-oauth migration 0001: the one new table this crate
-- owns. Each row links a third-party provider account to a local
-- user. Composite primary key (provider, provider_sub) is the
-- only safe natural key here: `provider_sub` is the immutable
-- subject the provider returns (GitHub `id`, Google `sub`), and
-- providers reuse short integers — `42` on GitHub and `42` on
-- Google are two different humans. Email is **not** the key; users
-- change emails at the provider all the time and we re-resolve on
-- every sign-in (see Hard rule R3 + the email-change-as-security-
-- event decision in SCOPE.md).
--
-- `ON DELETE CASCADE` on the user FK is deliberate: deleting a
-- starter user removes their linked identities in the same
-- transaction so an orphaned identity row cannot resurrect a
-- deleted account on next sign-in.
--
-- Migration 0002 (the users-password-optional rebuild) renames the
-- users table during its 12-step rebuild; the FK below targets the
-- table *name* so it reconnects automatically. The
-- `PRAGMA foreign_key_check` at the end of 0002 is the guard.
CREATE TABLE IF NOT EXISTS starter_auth_oauth_identities (
    provider      TEXT NOT NULL,
    provider_sub  TEXT NOT NULL,
    user_id       TEXT NOT NULL,
    email         TEXT,
    display_name  TEXT,
    linked_at     TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (provider, provider_sub),
    FOREIGN KEY (user_id) REFERENCES starter_auth_users_users(id) ON DELETE CASCADE
);

-- Lookup-by-user path: `GET /auth/oauth/identities` and the
-- `LinkedProvidersLookup` impl both scan by `user_id`. Without this
-- index those queries are full table scans.
CREATE INDEX IF NOT EXISTS idx_oauth_identities_user
    ON starter_auth_oauth_identities(user_id);
