-- G3 — Grants API. See sqlite/0006 for the rationale; this is
-- the postgres mirror. Postgres permits combining the two ALTERs
-- but we keep them on separate statements to mirror the sqlite
-- migration for readability.

ALTER TABLE starter_authz_rules ADD COLUMN source TEXT NOT NULL DEFAULT 'manual';
ALTER TABLE starter_authz_rules ADD COLUMN resource_id TEXT;
CREATE INDEX idx_starter_authz_rules_grant_instance
    ON starter_authz_rules (resource, resource_id, tenant_id)
    WHERE source = 'grant';
