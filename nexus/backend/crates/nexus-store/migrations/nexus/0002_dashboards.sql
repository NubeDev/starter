-- Dashboards and their panels. Both tenant-scoped and RLS-isolated like
-- nexus_datasources. Grants and panel references key on the immutable `id`; the
-- dashboard `slug` is a mutable route alias only, so renaming never orphans a
-- grant or a shared link.

CREATE TABLE nexus_dashboards (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id  text NOT NULL,
    slug       text NOT NULL,
    name       text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    -- A slug identifies a dashboard within a tenant; it may repeat across
    -- tenants, so uniqueness is per-tenant, not global.
    UNIQUE (tenant_id, slug)
);

ALTER TABLE nexus_dashboards ENABLE ROW LEVEL SECURITY;
ALTER TABLE nexus_dashboards FORCE ROW LEVEL SECURITY;
CREATE POLICY nexus_dashboards_tenant_isolation ON nexus_dashboards
    USING (tenant_id = current_setting('app.tenant_id', true))
    WITH CHECK (tenant_id = current_setting('app.tenant_id', true));
GRANT SELECT, INSERT, UPDATE, DELETE ON nexus_dashboards TO nexus_runtime;

CREATE TABLE nexus_panels (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id     text NOT NULL,
    dashboard_id  uuid NOT NULL REFERENCES nexus_dashboards(id) ON DELETE CASCADE,
    datasource_id uuid REFERENCES nexus_datasources(id) ON DELETE SET NULL,
    title         text NOT NULL,
    -- The panel's query against its datasource, run under the R4 guards.
    sql           text NOT NULL,
    -- Visualization kind (line/bar/table/stat …); the frontend renders it.
    viz           text NOT NULL DEFAULT 'table',
    -- Grid position/size as the canvas stores it; opaque to the backend.
    layout        jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at    timestamptz NOT NULL DEFAULT now()
);

ALTER TABLE nexus_panels ENABLE ROW LEVEL SECURITY;
ALTER TABLE nexus_panels FORCE ROW LEVEL SECURITY;
CREATE POLICY nexus_panels_tenant_isolation ON nexus_panels
    USING (tenant_id = current_setting('app.tenant_id', true))
    WITH CHECK (tenant_id = current_setting('app.tenant_id', true));
GRANT SELECT, INSERT, UPDATE, DELETE ON nexus_panels TO nexus_runtime;

CREATE INDEX nexus_panels_dashboard_idx ON nexus_panels (dashboard_id);
