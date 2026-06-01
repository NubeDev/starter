-- G3 — Grants API. `source` marks rule provenance so the drawer
-- can tell hand-written legacy rules apart from grants written
-- by `POST /v1/authz/grants`. `resource_id` lets a rule target
-- a specific instance of a kind (e.g. one dashboard page); rules
-- where it is NULL still apply kind-wide as before.
--
-- Sqlite only supports one column per ALTER TABLE statement, so
-- the two additions are split. The partial index speeds up the
-- per-instance grant lookups the share-scope reconciler runs on
-- every PUT.

ALTER TABLE starter_authz_rules ADD COLUMN source TEXT NOT NULL DEFAULT 'manual';
ALTER TABLE starter_authz_rules ADD COLUMN resource_id TEXT;
CREATE INDEX idx_starter_authz_rules_grant_instance
    ON starter_authz_rules (resource, resource_id, tenant_id)
    WHERE source = 'grant';
