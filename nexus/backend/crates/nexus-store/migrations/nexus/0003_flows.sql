-- Saved ingestion flows. A flow is a long-running ArkFlow stream the control
-- plane runs on the tenant's behalf: an input, a pipeline, and an output, each
-- stored as the config jsonb the FlowManager hands to the engine. Tenant-scoped
-- and RLS-isolated exactly like datasources and dashboards; the FlowManager keys
-- running flows on the immutable id.

CREATE TABLE nexus_flows (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id  text NOT NULL,
    name       text NOT NULL,
    -- The ArkFlow input config (e.g. an http_poll url + interval). Opaque to the
    -- store; validated when the FlowManager builds the stream.
    input      jsonb NOT NULL,
    -- The pipeline's processor list (json_to_arrow, sql, …).
    pipeline   jsonb NOT NULL DEFAULT '[]'::jsonb,
    -- The output config (e.g. a postgres uri + table).
    output     jsonb NOT NULL,
    -- Whether the flow should run. The control plane starts enabled flows; a
    -- disabled flow is stored but not running.
    enabled    boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now(),
    -- A flow name identifies a flow within a tenant; unique per tenant, like a
    -- dashboard slug.
    UNIQUE (tenant_id, name)
);

ALTER TABLE nexus_flows ENABLE ROW LEVEL SECURITY;
ALTER TABLE nexus_flows FORCE ROW LEVEL SECURITY;
CREATE POLICY nexus_flows_tenant_isolation ON nexus_flows
    USING (tenant_id = current_setting('app.tenant_id', true))
    WITH CHECK (tenant_id = current_setting('app.tenant_id', true));
GRANT SELECT, INSERT, UPDATE, DELETE ON nexus_flows TO nexus_runtime;
