-- Phase 7a — tenant scope on rules. NULL = global; non-null
-- restricts the rule to the named tenant. See SCOPE-EXT.md R11.
ALTER TABLE starter_authz_rules ADD COLUMN IF NOT EXISTS tenant_id TEXT;

CREATE INDEX IF NOT EXISTS idx_authz_rules_tenant
    ON starter_authz_rules (tenant_id);
