-- Query history: one row per query a user ran from Explore or a panel editor,
-- so they can recall, re-run, and star past queries.
--
-- Tenant-scoped and RLS-isolated like the rest of the control plane (see
-- 0005_tags.sql): a tenant only ever sees its own history, enforced by the
-- policy below rather than an application `WHERE`. `user_id` is text because
-- users live in the starter identity layer outside this store — the row
-- references the subject without owning it.
--
-- Retention is bounded by the application (it trims to the newest N per user on
-- write) rather than by the schema, so this table stays a thin recent-history
-- ledger, not an audit log — durable change history is WS-12's changelog.
CREATE TABLE nexus_query_history (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id     text NOT NULL,
    -- The starter-identity subject who ran the query.
    user_id       text NOT NULL,
    -- The datasource the query ran against. Nullable for the dev single-source
    -- path, which has no registered datasource id.
    datasource_id uuid,
    -- The SQL as authored (pre-binding), so a recall re-runs the same text.
    sql           text NOT NULL,
    ran_at        timestamptz NOT NULL DEFAULT now(),
    -- Execution outcome, mirrored from QueryStats. NULL row_count/elapsed_ms
    -- with a non-NULL error means the run failed.
    elapsed_ms    bigint,
    row_count     bigint,
    error         text,
    -- A user-pinned favourite, surfaced first in the history drawer.
    starred       boolean NOT NULL DEFAULT false
);

ALTER TABLE nexus_query_history ENABLE ROW LEVEL SECURITY;
ALTER TABLE nexus_query_history FORCE ROW LEVEL SECURITY;
CREATE POLICY nexus_query_history_tenant_isolation ON nexus_query_history
    USING (tenant_id = current_setting('app.tenant_id', true))
    WITH CHECK (tenant_id = current_setting('app.tenant_id', true));
GRANT SELECT, INSERT, UPDATE, DELETE ON nexus_query_history TO nexus_runtime;

-- The history drawer reads "my recent runs, newest first". Tenant- and
-- user-leading so the RLS predicate and the per-user filter both ride the index,
-- ordered by ran_at for the newest-first scan.
CREATE INDEX nexus_query_history_recent_idx
    ON nexus_query_history (tenant_id, user_id, ran_at DESC);
