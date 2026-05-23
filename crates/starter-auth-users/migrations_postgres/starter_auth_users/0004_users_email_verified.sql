-- Add email_verified column. Postgres mirror of
-- 0004_users_email_verified.sql in migrations/starter_auth_users/.
--
-- Translation notes:
--   sqlite INTEGER NOT NULL DEFAULT 1 (a 0/1-encoded bool)
--   →     BOOLEAN NOT NULL DEFAULT TRUE
--
-- Admin-created and OAuth-created users default to true (the
-- operator or provider vouches for the address); signup-created
-- users get false until explicit verification.
ALTER TABLE starter_auth_users_users
  ADD COLUMN IF NOT EXISTS email_verified BOOLEAN NOT NULL DEFAULT TRUE;
