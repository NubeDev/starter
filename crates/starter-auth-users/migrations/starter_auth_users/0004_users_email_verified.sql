-- Add email_verified column. Admin-created and OAuth-created users
-- default to true (the operator or provider vouches for the address);
-- signup-created users get false until explicit verification (Phase 2).
ALTER TABLE starter_auth_users_users
  ADD COLUMN email_verified INTEGER NOT NULL DEFAULT 1;
