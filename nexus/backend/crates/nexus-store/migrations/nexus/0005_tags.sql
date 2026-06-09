-- Tags: tenant-scoped labels applied to any entity, in or out of this store.
--
-- A tag is a `key` plus an optional `value`. A bare label like `temp` is a key
-- with a NULL value; a key:value tag like `building=abc` carries the value. One
-- table serves every taggable noun (dashboards, datasources, flows, alert
-- rules, and entities owned elsewhere such as users and teams) because the
-- target is referenced by (entity_type, entity_id), not a foreign key.
--
-- `entity_id` is text, not uuid, on purpose: users and teams live in the
-- starter identity layer outside this crate, so a tag references them by id
-- without owning them or coupling to their tables. That same choice rules out a
-- DB cascade — when a dashboard is deleted its tags are swept by the store's
-- `delete_for_entity`, called from the dashboard/datasource delete paths.
--
-- Tenant-scoped and RLS-isolated like the rest of the control plane: a tenant
-- only ever sees and writes its own tags, enforced by the policy below rather
-- than an application `WHERE`.
CREATE TABLE nexus_tags (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id   text NOT NULL,
    -- The kind of thing tagged: 'dashboard' | 'datasource' | 'flow' |
    -- 'alert_rule' | 'user' | 'team' | … . Free text so a new taggable noun
    -- needs no migration; the API validates the set it accepts.
    entity_type text NOT NULL,
    -- The tagged entity's id, as text so ids owned by other layers (users,
    -- teams) fit alongside this store's uuids.
    entity_id   text NOT NULL,
    -- The label. `value` NULL means a bare tag ([zone, temp]); a set value
    -- means a key:value tag ({building: abc}).
    key         text NOT NULL,
    value       text,
    created_at  timestamptz NOT NULL DEFAULT now(),
    -- One value per key per entity: re-tagging a key updates it in place
    -- (upsert) rather than stacking duplicate rows.
    UNIQUE (tenant_id, entity_type, entity_id, key)
);

ALTER TABLE nexus_tags ENABLE ROW LEVEL SECURITY;
ALTER TABLE nexus_tags FORCE ROW LEVEL SECURITY;
CREATE POLICY nexus_tags_tenant_isolation ON nexus_tags
    USING (tenant_id = current_setting('app.tenant_id', true))
    WITH CHECK (tenant_id = current_setting('app.tenant_id', true));
GRANT SELECT, INSERT, UPDATE, DELETE ON nexus_tags TO nexus_runtime;

-- Forward lookup: "what tags does this entity have" — the read on every
-- entity detail view. Tenant-leading so the RLS predicate rides the index.
CREATE INDEX nexus_tags_entity_idx ON nexus_tags (tenant_id, entity_type, entity_id);

-- Reverse lookup: "which entities are tagged building=abc" and "what keys/values
-- exist" (filter chips, autocomplete). Tenant-leading for the same reason.
CREATE INDEX nexus_tags_key_idx ON nexus_tags (tenant_id, key, value);
