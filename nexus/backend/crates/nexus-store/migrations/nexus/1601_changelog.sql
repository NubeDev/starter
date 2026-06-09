-- Append-only change ledger (WS-12) — audit log + undo/redo on one substrate.
--
-- Ports the `starter-changelog-postgres` schema
-- (crates/starter-changelog-postgres/migrations/0001_init.sql) into the nexus
-- metadata DB and makes it **tenant-scoped + RLS-isolated** like every other
-- nexus table. The starter `starter_changes` table is not tenant-aware out of the
-- box (one shared log), so nexus adds a `tenant_id` column and an RLS policy bound
-- to the `app.tenant_id` GUC. The nexus recorder/changelog write and read this
-- table inside a `tenant_tx` (see crates/nexus-store/src/tenant_tx.rs), so the GUC
-- is always set and a caller can never see or write another tenant's rows.
--
-- `actor_model` is a generated column over `actor_meta->>'model'`, kept indexed
-- so the AI-agent log can filter by model without a per-row JSON extract — the
-- same shape the starter backend uses.

CREATE TABLE nexus_changes (
    id               text PRIMARY KEY,
    tenant_id        text NOT NULL,
    at               timestamptz NOT NULL,
    actor_kind       text NOT NULL,
    actor_id         text,
    actor_meta       jsonb,
    actor_model      text GENERATED ALWAYS AS (actor_meta->>'model') STORED,
    resource_kind    text NOT NULL,
    resource_id      text NOT NULL,
    resource_owner   text,
    resource_version bigint,
    op               text NOT NULL,
    before           jsonb,
    after            jsonb,
    patch            jsonb,
    group_id         text NOT NULL,
    correlation      text
);

ALTER TABLE nexus_changes ENABLE ROW LEVEL SECURITY;
ALTER TABLE nexus_changes FORCE ROW LEVEL SECURITY;
CREATE POLICY nexus_changes_tenant_isolation ON nexus_changes
    USING (tenant_id = current_setting('app.tenant_id', true))
    WITH CHECK (tenant_id = current_setting('app.tenant_id', true));
GRANT SELECT, INSERT, UPDATE, DELETE ON nexus_changes TO nexus_runtime;

-- Resource history timeline: the "History" tab on a dashboard/datasource pages
-- newest-first by (resource_kind, resource_id).
CREATE INDEX nexus_changes_resource_idx
    ON nexus_changes (tenant_id, resource_kind, resource_id, at DESC);

-- Actor scan: powers `undo` (newest group authored by the actor) and the audit
-- "by user / by agent model" filters.
CREATE INDEX nexus_changes_actor_idx
    ON nexus_changes (tenant_id, actor_kind, actor_id, at DESC);
CREATE INDEX nexus_changes_actor_model_idx
    ON nexus_changes (tenant_id, actor_kind, actor_model, at DESC);

-- Group load: `undo`/`redo` replay a whole transaction by group_id.
CREATE INDEX nexus_changes_group_idx
    ON nexus_changes (tenant_id, group_id);

-- Audit list default ordering + keyset pagination cursor.
CREATE INDEX nexus_changes_at_idx
    ON nexus_changes (tenant_id, at DESC, id DESC);
