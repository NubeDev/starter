-- rubix-side seed for `changelog_kind_policy` — `tenant` kind.
--
-- Follow-up to `0001_audit_floor_seed.sql`, which pinned
-- `user` and `team`. Tenants are identity boundaries (they
-- control per-tenant data visibility) and their lifecycle is
-- as security-relevant as user role / disable. Same audit-floor
-- posture: `max_age_days = NULL` keeps the change rows for the
-- `tenant` kind forever, immune to any future per-kind sweep.
--
-- See `rubix/docs/proposal/audit-log.md` for the rationale.
--
-- `ON CONFLICT DO NOTHING` so the migration is safe to re-run
-- and so an operator who manually flipped `tenant` back to
-- bounded retention is not overwritten by a routine re-deploy.

INSERT INTO changelog_kind_policy (resource_kind, max_age_days) VALUES
    ('tenant', NULL)
ON CONFLICT (resource_kind) DO NOTHING;
