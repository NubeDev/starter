-- starter-auth-users tenant hierarchy (ADR-tenant-hierarchy).
--
-- Tenants become recursive: a tenant may have a parent tenant. A
-- parent transitively administers and sees every descendant tenant;
-- siblings and parents stay isolated from a child by construction
-- (the authz cross-tenant predicate admits only a principal's own
-- tenant + its subtree).
--
-- `parent_id` is the source-of-truth edge (NULL = a root tenant).
-- A closure table holds the derived transitive closure so the authz
-- hot path can answer "is X an ancestor of Y?" as an indexed point
-- lookup instead of a recursive walk on every check.
--
-- Re-parenting is deliberately unsupported (mirrors immutable slugs
-- + deferred tenant deletion); the parent edge is set at create and
-- never moved. A trigger enforces that immutability.

ALTER TABLE starter_auth_users_tenants
    ADD COLUMN parent_id TEXT
        REFERENCES starter_auth_users_tenants(id) ON DELETE RESTRICT;

CREATE INDEX IF NOT EXISTS idx_tenants_parent
    ON starter_auth_users_tenants (parent_id);

-- Transitive closure. One row per (ancestor, descendant) pair,
-- INCLUDING the self-pair at depth 0 — so "the subtree of X" is
-- simply `SELECT descendant_id WHERE ancestor_id = X`, and X is in
-- its own subtree. `depth` is 0 for self, 1 for a direct child, etc.
CREATE TABLE IF NOT EXISTS starter_auth_users_tenant_closure (
    ancestor_id   TEXT NOT NULL REFERENCES starter_auth_users_tenants(id) ON DELETE CASCADE,
    descendant_id TEXT NOT NULL REFERENCES starter_auth_users_tenants(id) ON DELETE CASCADE,
    depth         INTEGER NOT NULL,
    PRIMARY KEY (ancestor_id, descendant_id)
);

CREATE INDEX IF NOT EXISTS idx_tenant_closure_descendant
    ON starter_auth_users_tenant_closure (descendant_id);

-- Backfill: every existing tenant is a root (no parent edges exist
-- yet), so each gets exactly its depth-0 self row. Idempotent via
-- the PK + the NOT-EXISTS guard.
INSERT INTO starter_auth_users_tenant_closure (ancestor_id, descendant_id, depth)
SELECT id, id, 0 FROM starter_auth_users_tenants t
WHERE NOT EXISTS (
    SELECT 1 FROM starter_auth_users_tenant_closure c
    WHERE c.ancestor_id = t.id AND c.descendant_id = t.id
);

-- parent_id is immutable after create (re-parenting unsupported).
CREATE TRIGGER IF NOT EXISTS trg_tenants_parent_immutable
BEFORE UPDATE OF parent_id ON starter_auth_users_tenants
FOR EACH ROW
WHEN OLD.parent_id IS NOT NEW.parent_id
BEGIN
    SELECT RAISE(ABORT, 'starter_auth_users_tenants: parent_id is immutable');
END;
