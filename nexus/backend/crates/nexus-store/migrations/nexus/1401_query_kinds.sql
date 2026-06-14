-- Tenant-authored query-kinds. A *query-kind* is a named, reusable SQL query an
-- admin promotes out of an Explore session: instead of pasting raw SQL into a
-- panel, callers invoke the kind by name and pass declared params. This table
-- holds the tenant's own kinds — the durable record the dispatcher resolves on a
-- registry miss.
--
-- These rows are tenant-scoped and RLS-isolated exactly like agents/flows: a
-- tenant only ever sees and authors its own kinds, enforced by the policy below
-- rather than an application `WHERE`. The *built-in* file pack of kinds is global
-- and ships with the binary — it lives outside the DB and is not represented
-- here; this table is strictly the tenant-authored overlay.
--
-- Names are reverse-DNS (e.g. com.acme.meters_list), like an agent or flow name,
-- and unique within a tenant. The API layer lint-validates the SQL before it
-- inserts a row — it checks the declared `tables` are tenant-guarded and every
-- `$param` is declared — so a row that lands here is already safe to bind and
-- run; the store only persists it, it does not re-validate.

CREATE TABLE nexus_query_kinds (
    id                 uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id          text NOT NULL,
    -- Reverse-DNS id a request invokes (e.g. com.acme.foo). Identifies the kind
    -- within a tenant, like an agent or flow name.
    name               text NOT NULL,
    -- The raw SQL template. Carries `$caller_tenant_id`, `$__time*` macros, and
    -- `$param` references — all bound by the shared binder, never inlined. The
    -- API lint guaranteed this is safe before insert.
    sql                text NOT NULL,
    -- The JSON Schema document for this kind's params: the contract a request's
    -- params validate against, and the source of declared defaults.
    params_schema      jsonb NOT NULL DEFAULT '{}'::jsonb,
    -- The datasource shape this kind targets (e.g. "postgres").
    datasource_kind    text NOT NULL,
    -- Tables the kind reads. The lint guaranteed each tenant-scoped table here is
    -- guarded by `$caller_tenant_id` in the SQL.
    tables             text[] NOT NULL DEFAULT '{}',
    -- Optional pinned datasource id; NULL means any datasource of
    -- `datasource_kind` the caller can view.
    datasource_binding text,
    -- Optional human description for the picker UI.
    description        text,
    created_at         timestamptz NOT NULL DEFAULT now(),
    -- A kind name identifies a kind within a tenant, like a flow name.
    UNIQUE (tenant_id, name)
);

ALTER TABLE nexus_query_kinds ENABLE ROW LEVEL SECURITY;
ALTER TABLE nexus_query_kinds FORCE ROW LEVEL SECURITY;
CREATE POLICY nexus_query_kinds_tenant_isolation ON nexus_query_kinds
    USING (tenant_id = current_setting('app.tenant_id', true))
    WITH CHECK (tenant_id = current_setting('app.tenant_id', true));
GRANT SELECT, INSERT, UPDATE, DELETE ON nexus_query_kinds TO nexus_runtime;
