-- rubix-side seed for `changelog_kind_policy`.
--
-- The table itself is provisioned by the `changelog` migration
-- source (see
-- `crates/starter-changelog-postgres/migrations/0004_changelog_kind_policy.sql`).
-- This rubix-owned migration seeds the *policy* — which kinds are
-- pinned to the audit floor.
--
-- A row with `max_age_days = NULL` declares the kind explicitly
-- exempt from any future automatic sweep. The intent is recorded
-- in SQL rather than in tribal memory so a future operator who
-- adds finite retention for chatty kinds cannot accidentally
-- prune the security-relevant audit trail.
--
-- See `rubix/docs/proposal/audit-log.md` for the rationale.
--
-- `ON CONFLICT DO NOTHING` so the migration is safe to re-run and
-- so an operator who manually flipped a kind back to bounded
-- retention is not overwritten by a routine re-deploy. Promoting
-- the floor is intentional; demoting it is also intentional, and
-- both should be operator-driven UPDATEs rather than seed
-- migrations bouncing the value.

INSERT INTO changelog_kind_policy (resource_kind, max_age_days) VALUES
    ('user', NULL),
    ('team', NULL)
ON CONFLICT (resource_kind) DO NOTHING;
