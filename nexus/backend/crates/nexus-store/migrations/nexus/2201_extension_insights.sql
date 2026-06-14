-- Extension-contributed insights (RW-07) — the global insight registry an
-- installed extension contributes via `contributes.insights[]`, the dual of
-- `nexus_extension_query_kinds` (1801) for the insight stage rather than the
-- query stage.
--
-- An insight is a named, reusable, sandboxed post-query transform script (see
-- 2101_insights.sql for the tenant-authored overlay). Like extension
-- query-kinds these rows are **global, not tenant-scoped**: an extension is
-- installed once for the whole deployment (admin-gated — WS-14 §9 Q5), so its
-- insights are available to every tenant exactly like the file-pack query-kinds
-- are. There is no `tenant_id` column and no RLS policy; the table is keyed by a
-- globally-unique `name`. The script runs in the Rhai sandbox at query time
-- against the caller's own result rows, so a global definition still only ever
-- touches the caller's data.
--
-- The host compiles each script against the insight sandbox before a row lands
-- here (the contribution path rejects a non-compiling script), exactly as the
-- query-kind lint gates 1801; the store only persists.

CREATE TABLE nexus_extension_insights (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    -- The owning extension's reverse-DNS id (e.g. com.nexus.hello). The cleanup
    -- provider deletes every row with this id on uninstall+purge.
    extension_id  text NOT NULL,
    -- Reverse-DNS insight name a request invokes (e.g. com.nexus.hello.zscore).
    -- Globally unique: a request resolving `name` must map to one definition
    -- regardless of which tenant calls it.
    name          text NOT NULL,
    -- The Rhai transform body. Compiled against the insight sandbox before this
    -- row was written; the store does not re-validate.
    script        text NOT NULL,
    -- Advisory JSON-Schema for the script's params (UI only); the sandbox is the
    -- safety boundary, so it is nullable.
    params_schema jsonb,
    created_at    timestamptz NOT NULL DEFAULT now(),
    UNIQUE (name)
);

-- Cleanup deletes by owner; index the lookup so uninstall is cheap.
CREATE INDEX nexus_extension_insights_by_extension
    ON nexus_extension_insights (extension_id);

-- No RLS: these rows are global config, not tenant data (see header). The
-- runtime role needs full DML — the install/uninstall path writes, the query
-- path reads, both under the deployment's single global view.
GRANT SELECT, INSERT, UPDATE, DELETE ON nexus_extension_insights TO nexus_runtime;
