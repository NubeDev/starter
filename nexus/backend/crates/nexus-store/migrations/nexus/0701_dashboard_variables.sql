-- Dashboard variables (WS-02). Tenant-scoped and RLS-isolated like dashboards
-- and panels. A variable is dashboard-scoped: its definition (kind + authoring
-- config) and current selection live here so one dashboard can be parameterised
-- and re-scoped across a whole fleet. Resolved values flow into a query through
-- the WS-03 binder as bound args, never inlined.
--
-- Persisted relationally (rather than in a dashboard JSONB blob) because the C1
-- dashboard JSON model is not yet built; when WS-05 lands import/export it folds
-- this table into the serialised dashboard shape.

CREATE TABLE nexus_dashboard_variables (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id     text NOT NULL,
    dashboard_id  uuid NOT NULL REFERENCES nexus_dashboards(id) ON DELETE CASCADE,
    -- Reference name without the `$`, unique within a dashboard.
    name          text NOT NULL,
    -- Human label for the variable bar; null falls back to `name`.
    label         text,
    -- One of: constant | custom | query | datasource | interval | textbox.
    kind          text NOT NULL,
    -- Kind-specific authoring config (the custom list, option SQL + datasource,
    -- interval steps, datasource-kind filter). Opaque to the backend; the UI
    -- owns each shape per kind.
    options_config jsonb NOT NULL DEFAULT '{}'::jsonb,
    -- Currently selected value(s): one entry single-select, several multi.
    current       jsonb NOT NULL DEFAULT '[]'::jsonb,
    multi         boolean NOT NULL DEFAULT false,
    include_all   boolean NOT NULL DEFAULT false,
    hidden        boolean NOT NULL DEFAULT false,
    -- Display/resolution order in the bar; lower first, ties by created_at.
    sort_order    integer NOT NULL DEFAULT 0,
    created_at    timestamptz NOT NULL DEFAULT now(),
    -- A variable name identifies it within one dashboard; it may repeat across
    -- dashboards (and tenants), so uniqueness is per dashboard.
    UNIQUE (dashboard_id, name)
);

ALTER TABLE nexus_dashboard_variables ENABLE ROW LEVEL SECURITY;
ALTER TABLE nexus_dashboard_variables FORCE ROW LEVEL SECURITY;
CREATE POLICY nexus_dashboard_variables_tenant_isolation ON nexus_dashboard_variables
    USING (tenant_id = current_setting('app.tenant_id', true))
    WITH CHECK (tenant_id = current_setting('app.tenant_id', true));
GRANT SELECT, INSERT, UPDATE, DELETE ON nexus_dashboard_variables TO nexus_runtime;

CREATE INDEX nexus_dashboard_variables_dashboard_idx
    ON nexus_dashboard_variables (dashboard_id, sort_order);
