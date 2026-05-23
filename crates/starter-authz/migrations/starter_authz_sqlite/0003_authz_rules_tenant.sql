-- Phase 7a — tenant scope on rules. NULL = global rule (applies
-- to every tenant); non-null = only matches a principal bound to
-- that tenant. See DOCS/auth/authz/SCOPE-EXT.md R11.
ALTER TABLE starter_authz_rules ADD COLUMN tenant_id TEXT;

CREATE INDEX IF NOT EXISTS idx_authz_rules_tenant
    ON starter_authz_rules (tenant_id);
