-- Stored insights (RW-06). A tenant-scoped, named post-query transform script.
-- Panels reference an insight by id; the query path resolves it, authorises the
-- tenant, and runs it under the Rhai sandbox before serializing the result.
--
-- Tenant-isolated by RLS exactly like dashboards/folders: every row carries a
-- tenant_id and the policy scopes both reads and writes to the current tenant.
-- `params_schema` is advisory JSON-Schema for the UI; the sandbox enforces
-- safety regardless of it, so it is nullable.

CREATE TABLE nexus_insights (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id     text NOT NULL,
    name          text NOT NULL,
    script        text NOT NULL,
    params_schema jsonb,
    created_at    timestamptz NOT NULL DEFAULT now()
);

ALTER TABLE nexus_insights ENABLE ROW LEVEL SECURITY;
ALTER TABLE nexus_insights FORCE ROW LEVEL SECURITY;
CREATE POLICY nexus_insights_tenant_isolation ON nexus_insights
    USING (tenant_id = current_setting('app.tenant_id', true))
    WITH CHECK (tenant_id = current_setting('app.tenant_id', true));
GRANT SELECT, INSERT, UPDATE, DELETE ON nexus_insights TO nexus_runtime;
