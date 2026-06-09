-- Dashboard folders (WS-05). Tenant-scoped and RLS-isolated like dashboards.
-- Folders are nestable via a self-referential parent: a NULL parent_id is a
-- root folder. Deleting a folder detaches (does not cascade-delete) its
-- children and any dashboards filed under it — losing the organisation must
-- never silently destroy the dashboards themselves, so the references use
-- ON DELETE SET NULL, leaving orphaned items at the root.

CREATE TABLE nexus_folders (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id  text NOT NULL,
    -- Self-reference for nesting; NULL is a root folder. A folder cannot be
    -- filed under one in another tenant because RLS scopes the candidate rows.
    parent_id  uuid REFERENCES nexus_folders(id) ON DELETE SET NULL,
    name       text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

ALTER TABLE nexus_folders ENABLE ROW LEVEL SECURITY;
ALTER TABLE nexus_folders FORCE ROW LEVEL SECURITY;
CREATE POLICY nexus_folders_tenant_isolation ON nexus_folders
    USING (tenant_id = current_setting('app.tenant_id', true))
    WITH CHECK (tenant_id = current_setting('app.tenant_id', true));
GRANT SELECT, INSERT, UPDATE, DELETE ON nexus_folders TO nexus_runtime;

CREATE INDEX nexus_folders_parent_idx ON nexus_folders (parent_id);
