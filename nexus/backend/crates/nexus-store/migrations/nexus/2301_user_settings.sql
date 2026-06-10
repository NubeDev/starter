-- Per-user settings (freeform JSON) — the caller's personal, tenant-scoped
-- workspace state that doesn't warrant its own typed column.
--
-- Unlike `starter_prefs_user` (1501_prefs.sql), which is a fixed-column schema
-- owned by the reused `starter-prefs` crate (units, locale, theme), this is a
-- deliberately open `jsonb` bag for nexus-side UI state: starred dashboards,
-- collapsed sidebar groups, last-opened tab — the "random stuff" a SPA wants to
-- remember per user without a migration each time. The shape is the frontend's
-- contract, not the DB's; the column is opaque here.
--
-- Keyed by (tenant_id, user_id): a user has exactly one settings row per tenant
-- it is a member of. `user_id` is the principal subject. Tenant-isolated by the
-- same `app.tenant_id` RLS GUC as the rest of the control plane — every access
-- runs inside a `tenant_tx`, so a caller only ever sees its own tenant's rows;
-- the route layer additionally pins `user_id = principal.subject` so one user
-- cannot read or write another's row within a shared tenant.
CREATE TABLE nexus_user_settings (
    tenant_id  text NOT NULL,
    user_id    text NOT NULL,
    -- Freeform UI state. `{}` is a valid empty bag; the frontend owns the keys.
    settings   jsonb NOT NULL DEFAULT '{}'::jsonb,
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, user_id)
);

ALTER TABLE nexus_user_settings ENABLE ROW LEVEL SECURITY;
ALTER TABLE nexus_user_settings FORCE ROW LEVEL SECURITY;
CREATE POLICY nexus_user_settings_tenant_isolation ON nexus_user_settings
    USING (tenant_id = current_setting('app.tenant_id', true))
    WITH CHECK (tenant_id = current_setting('app.tenant_id', true));
GRANT SELECT, INSERT, UPDATE, DELETE ON nexus_user_settings TO nexus_runtime;
