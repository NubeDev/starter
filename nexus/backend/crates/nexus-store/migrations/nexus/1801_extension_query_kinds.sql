-- Extension-contributed query-kinds (WS-14) — the *third source* the kinds
-- dispatcher resolves, beside the built-in file pack (global, ships with the
-- binary) and the tenant-authored overlay (`nexus_query_kinds`, RLS-isolated).
--
-- A query-kind is a named, reusable SQL query (see 1401_query_kinds.sql). WS-10
-- §9 Q1 settles the extension-contribution path as a *source*, not a
-- materialize-into-`nexus_query_kinds` step: an installed extension's
-- `contributes.warehouse_templates[]` land here, the dispatcher consults this
-- table on a file-pack miss (before the tenant overlay, since extension kinds
-- are global/admin-curated like the file pack), and uninstall just deletes the
-- rows this extension owns. That keeps the file-pack / tenant-DB / extension
-- boundaries clean — cleanup drops the source, it does not surgically unpick
-- per-tenant rows.
--
-- Crucially these rows are **global, not tenant-scoped**: an extension is
-- installed once for the whole deployment (admin-gated — WS-14 §9 Q5), so its
-- kinds apply to every tenant exactly like the file pack does. There is no
-- `tenant_id` column and no RLS policy; the table is keyed by
-- `(extension_id, name)`. The dispatcher still binds `$caller_tenant_id` at run
-- time, so a kind reading a tenant-scoped table is filtered to the *caller's*
-- tenant — global definition, per-caller data, identical to a file-pack kind.
--
-- The lint guarantees (declared `$param`s, `$caller_tenant_id`-guarded
-- tenant-scoped `tables`) are enforced by the install/contribution path before a
-- row lands here, exactly as for the file pack and the tenant overlay; the store
-- only persists, it does not re-validate.

CREATE TABLE nexus_extension_query_kinds (
    id                 uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    -- The owning extension's reverse-DNS id (e.g. com.nubeio.notes). The
    -- cleanup provider deletes every row with this id on uninstall+purge.
    extension_id       text NOT NULL,
    -- Reverse-DNS kind id a request invokes (e.g. com.acme.foo). Globally
    -- unique because extension kinds are global; two extensions cannot
    -- contribute the same kind name (the UNIQUE below rejects the second).
    name               text NOT NULL,
    -- The raw SQL template. Carries `$caller_tenant_id`, `$__time*` macros, and
    -- `$param` references — all bound by the shared binder, never inlined.
    sql                text NOT NULL,
    -- The JSON Schema document for this kind's params.
    params_schema      jsonb NOT NULL DEFAULT '{}'::jsonb,
    -- The datasource shape this kind targets (e.g. "postgres").
    datasource_kind    text NOT NULL,
    -- Tables the kind reads. The lint guaranteed each tenant-scoped table here
    -- is guarded by `$caller_tenant_id` in the SQL.
    tables             text[] NOT NULL DEFAULT '{}',
    -- Optional pinned datasource id; NULL means any datasource of
    -- `datasource_kind` the caller can view.
    datasource_binding text,
    -- Optional human description for the picker UI.
    description        text,
    created_at         timestamptz NOT NULL DEFAULT now(),
    -- A kind name is globally unique: a request resolving `name` must map to one
    -- definition regardless of which tenant calls it.
    UNIQUE (name)
);

-- Cleanup deletes by owner; index the lookup so uninstall is cheap.
CREATE INDEX nexus_extension_query_kinds_by_extension
    ON nexus_extension_query_kinds (extension_id);

-- No RLS: these rows are global config, not tenant data (see header). The
-- runtime role needs full DML — the install/uninstall path writes, the
-- dispatcher reads, both under the deployment's single global view.
GRANT SELECT, INSERT, UPDATE, DELETE ON nexus_extension_query_kinds TO nexus_runtime;
