-- Navigation tree (WS-13). Tenant-scoped and RLS-isolated, mirroring
-- `0602_folders.sql`. A nav node is the unit a user navigates to *and* the unit
-- access is granted on (per WS-13 §6) — distinct from a folder, which only
-- *files* a dashboard once. A nav node *mounts* a (possibly shared) page with a
-- context, so the same dashboard legitimately appears under many nodes.
--
-- Nodes nest via a self-referential parent: a NULL parent_id is a root node.
-- Deleting a node re-roots its children rather than cascading — losing a branch
-- header must never silently destroy the nodes filed under it, so the reference
-- uses ON DELETE SET NULL.
--
-- `target` is a JSONB tagged union, never a typed FK column:
--   { "kind": "group" }                          -- non-clickable header
--   { "kind": "dashboard", "dashboardId": "…" }  -- a reusable page mount
--   { "kind": "route", "route": "agents" }       -- a static app page
-- A dashboard target's id is validated in the handler against a tenant-scoped
-- SELECT (not a DB FK): `nexus_dashboards.id` is a global PK with `tenant_id` a
-- separate column (0002_dashboards.sql), so a bare REFERENCES would let a node
-- point at another tenant's dashboard. On dashboard delete the owning store
-- sweeps dependent nodes back to `{ "kind": "group" }` (see dashboard/delete.rs).
--
-- `context` is dashboard-targets-only and is EXACTLY { values?, tags? } per the
-- §1 merge contract. It is opaque JSONB here; the handler/UI own its shape.

CREATE TABLE nexus_nav_nodes (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id  text NOT NULL,
    -- Self-reference for nesting; NULL is a root node. A node cannot be filed
    -- under one in another tenant because RLS scopes the candidate rows.
    parent_id  uuid REFERENCES nexus_nav_nodes(id) ON DELETE SET NULL,
    title      text NOT NULL,
    sort_order integer NOT NULL DEFAULT 0,
    target     jsonb NOT NULL DEFAULT '{"kind":"group"}'::jsonb,
    -- Dashboard-target context payload ({ values?, tags? }); NULL for group/route.
    context    jsonb,
    icon       text,
    accent     text,
    created_at timestamptz NOT NULL DEFAULT now()
);

ALTER TABLE nexus_nav_nodes ENABLE ROW LEVEL SECURITY;
ALTER TABLE nexus_nav_nodes FORCE ROW LEVEL SECURITY;
CREATE POLICY nexus_nav_nodes_tenant_isolation ON nexus_nav_nodes
    USING (tenant_id = current_setting('app.tenant_id', true))
    WITH CHECK (tenant_id = current_setting('app.tenant_id', true));
GRANT SELECT, INSERT, UPDATE, DELETE ON nexus_nav_nodes TO nexus_runtime;

-- The tree is read top-down ordered by (parent, sort_order); tenant-leading so
-- the RLS predicate rides the index.
CREATE INDEX nexus_nav_nodes_tree_idx
    ON nexus_nav_nodes (tenant_id, parent_id, sort_order);
