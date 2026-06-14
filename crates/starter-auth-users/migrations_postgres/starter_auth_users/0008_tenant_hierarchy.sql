-- starter-auth-users tenant hierarchy (ADR-tenant-hierarchy).
-- Postgres mirror of 0008_tenant_hierarchy.sql in
-- migrations/starter_auth_users/.
--
-- Tenants become recursive: a tenant may have a parent tenant. A
-- parent transitively administers + sees every descendant tenant;
-- siblings and parents stay isolated from a child by construction.
-- `parent_id` is the source-of-truth edge (NULL = root); the closure
-- table is the derived transitive closure for O(1) ancestor lookups
-- on the authz hot path. Re-parenting is unsupported — a trigger
-- enforces parent_id immutability (raises SQLSTATE 23514 like the
-- slug/tenant immutability triggers, so callers detect check_violation).

ALTER TABLE starter_auth_users_tenants
    ADD COLUMN IF NOT EXISTS parent_id TEXT
        REFERENCES starter_auth_users_tenants(id) ON DELETE RESTRICT;

CREATE INDEX IF NOT EXISTS idx_tenants_parent
    ON starter_auth_users_tenants (parent_id);

-- Transitive closure. One row per (ancestor, descendant) pair,
-- INCLUDING the self-pair at depth 0.
CREATE TABLE IF NOT EXISTS starter_auth_users_tenant_closure (
    ancestor_id   TEXT NOT NULL REFERENCES starter_auth_users_tenants(id) ON DELETE CASCADE,
    descendant_id TEXT NOT NULL REFERENCES starter_auth_users_tenants(id) ON DELETE CASCADE,
    depth         INTEGER NOT NULL,
    PRIMARY KEY (ancestor_id, descendant_id)
);

CREATE INDEX IF NOT EXISTS idx_tenant_closure_descendant
    ON starter_auth_users_tenant_closure (descendant_id);

-- Backfill: every existing tenant is a root → one depth-0 self row.
INSERT INTO starter_auth_users_tenant_closure (ancestor_id, descendant_id, depth)
SELECT id, id, 0 FROM starter_auth_users_tenants t
ON CONFLICT (ancestor_id, descendant_id) DO NOTHING;

CREATE OR REPLACE FUNCTION starter_auth_users_tenants_parent_immutable()
RETURNS trigger AS $$
BEGIN
    IF NEW.parent_id IS DISTINCT FROM OLD.parent_id THEN
        RAISE EXCEPTION
            'starter_auth_users_tenants: parent_id is immutable'
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_tenants_parent_immutable
    ON starter_auth_users_tenants;
CREATE TRIGGER trg_tenants_parent_immutable
BEFORE UPDATE ON starter_auth_users_tenants
FOR EACH ROW
EXECUTE FUNCTION starter_auth_users_tenants_parent_immutable();
