-- starter-auth-oauth migration 0002: relax NOT NULL on
-- starter_auth_users_users.password_hash so OAuth-only users can land
-- with NULL. Postgres flavour: one ALTER COLUMN, no rebuild.
--
-- Ships in starter-auth-oauth (not starter-auth-users) so consumers
-- who never enable OAuth never run it (Hard rule R8).
ALTER TABLE starter_auth_users_users
    ALTER COLUMN password_hash DROP NOT NULL;
