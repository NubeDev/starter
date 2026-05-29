-- Bootstrap admin allow-all rule. Seeded atomically with the
-- schema so the first request after the `DbPolicyEngine` swap
-- does not lock the operator out of the admin UI that writes
-- rules. See `rubix/docs/proposal/access-control-redesign.md`
-- §0.3 step 2 + §0.4 "Lockout risk".
--
-- The rule is global (`tenant_id = NULL`), targets every
-- resource (`*`) and every action (`["*"]`), and only fires
-- for principals already carrying `Role::Admin`. The Effect /
-- Role enums serialize lowercase (see
-- `crates/starter-authz/src/config.rs#Effect` and
-- `crates/starter-spi/src/auth/role.rs#Role`).
--
-- Idempotent: `ON CONFLICT (id) DO NOTHING` so re-running the
-- migrator after a manual operator-authored admin rule with a
-- different id is a no-op rather than a duplicate. The literal
-- id is stable so a future migration can target / replace it.
INSERT INTO starter_authz_rules
    (id, role, resource, actions, condition, effect, priority, created_by, tenant_id)
VALUES
    ('bootstrap-admin-allow-all', 'admin', '*', '["*"]'::jsonb, NULL, 'allow', 0, 'migration', NULL)
ON CONFLICT (id) DO NOTHING;
