-- Promote `'system'` from a reserved-slug pseudo-tenant into a real
-- tenant row. The bundled rubix dashboards (and any other code that
-- writes rows under `tenant_id = 'system'`) need a matching
-- `starter_auth_users_tenants` row so memberships and session
-- bindings can reference it via foreign key.
--
-- Why now: the auth flow binds every session to a tenant (`Phase 7a`)
-- and any logged-in user — including an Admin — must end up with a
-- principal `tenant_id` that resolves to either a real tenants row
-- or the super-admin sentinel `"*"`. Before this migration, cookie
-- login of an Admin user produced a `tenant_id = NULL` session which
-- the dashboard SSE then filtered to an empty snapshot.

-- The 0005 migration defined `CHECK (slug NOT IN (..., 'system') ...)`
-- as an unnamed constraint; Postgres auto-named it
-- `starter_auth_users_tenants_check`. Drop it and re-add without
-- `'system'`. The DROP/ADD pair is idempotent so a partial-replay
-- of this migration (e.g. crash after ADD before COMMIT) still
-- converges on the desired state.
ALTER TABLE starter_auth_users_tenants
    DROP CONSTRAINT IF EXISTS starter_auth_users_tenants_check;
ALTER TABLE starter_auth_users_tenants
    DROP CONSTRAINT IF EXISTS starter_auth_users_tenants_slug_check;

ALTER TABLE starter_auth_users_tenants
    ADD CONSTRAINT starter_auth_users_tenants_slug_check
    CHECK (
        slug NOT IN (
            'admin','api','auth','v1','v2','static','health',
            'metrics','openapi','extensions','mcp','tools',
            'default'
        )
        AND slug !~ '^[0-9]'
    );

INSERT INTO starter_auth_users_tenants (id, slug, display_name)
    VALUES ('system', 'system', 'System')
    ON CONFLICT (id) DO NOTHING;

-- Backfill: every existing Admin user gets membership in the system
-- tenant. Non-admins keep whatever memberships they already have.
INSERT INTO starter_auth_users_memberships (tenant_id, user_id, role)
SELECT 'system', id, 'admin'
  FROM starter_auth_users_users
 WHERE role = 'admin'
ON CONFLICT (tenant_id, user_id) DO NOTHING;
