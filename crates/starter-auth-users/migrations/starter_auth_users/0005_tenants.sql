-- Phase 7a — tenants + memberships (SCOPE-EXT.md R11/R12).
--
-- `slug` is the URL-facing tenant identifier. A reserved-slug list
-- is enforced at INSERT via a CHECK constraint so writes that race
-- past the application-level check still fail loudly. Adding more
-- reserved slugs later is one migration, not a schema rewrite.
--
-- Reserved (refused on INSERT): admin, api, auth, v1, v2, static,
-- health, metrics, openapi, extensions, mcp, tools, default,
-- system, and any all-digits slug.
CREATE TABLE IF NOT EXISTS starter_auth_users_tenants (
    id                  TEXT PRIMARY KEY,
    slug                TEXT NOT NULL UNIQUE,
    display_name        TEXT NOT NULL,
    -- Per-tenant override for the audit-log allow-sample rate.
    -- NULL = use the global STARTER_AUTHZ_DECISION_ALLOW_SAMPLE.
    audit_allow_sample  INTEGER,
    created_at          TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (
        slug NOT IN (
            'admin','api','auth','v1','v2','static','health',
            'metrics','openapi','extensions','mcp','tools',
            'default','system'
        )
        AND slug NOT GLOB '[0-9]*'
    )
);

CREATE TABLE IF NOT EXISTS starter_auth_users_memberships (
    tenant_id   TEXT NOT NULL REFERENCES starter_auth_users_tenants(id) ON DELETE CASCADE,
    user_id     TEXT NOT NULL REFERENCES starter_auth_users_users(id) ON DELETE CASCADE,
    role        TEXT NOT NULL CHECK (role IN ('reader','writer','admin')),
    created_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (tenant_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_memberships_user
    ON starter_auth_users_memberships (user_id);

-- Phase 7a — bind every API token to a single tenant. New tokens
-- MUST be issued with a tenant_id; the super-admin sentinel `*`
-- is reserved for cross-tenant admin tokens (issued only to users
-- with role=admin in the global users table).
--
-- For the migration we backfill `*` on every pre-existing token
-- row — pre-Phase-7 tokens were minted without a tenant binding
-- and the safest default is "treat them as cross-tenant admin"
-- since they predate the predicate. Operators who want to scope
-- existing tokens must revoke + reissue.
ALTER TABLE starter_auth_users_tokens ADD COLUMN tenant_id TEXT NOT NULL DEFAULT '*';

CREATE INDEX IF NOT EXISTS idx_tokens_tenant
    ON starter_auth_users_tokens (tenant_id);

-- (user_id, tenant_id) immutability — once a token row exists,
-- those columns may not be UPDATEd. SCOPE-EXT.md R12: "a
-- constraint in prose is not a constraint" — the trigger refuses
-- the UPDATE at the DB level so a buggy migration cannot silently
-- move a token to a different tenant.
CREATE TRIGGER IF NOT EXISTS trg_tokens_user_tenant_immutable
BEFORE UPDATE OF user_id, tenant_id ON starter_auth_users_tokens
FOR EACH ROW
WHEN OLD.user_id IS NOT NEW.user_id OR OLD.tenant_id IS NOT NEW.tenant_id
BEGIN
    SELECT RAISE(ABORT, 'starter_auth_users_tokens: (user_id, tenant_id) are immutable');
END;

-- Sessions carry a tenant binding too. New sessions get tenant_id
-- at issue time (single-membership auto-selects; multi-membership
-- renders the picker interstitial). Nullable for pre-Phase-7
-- sessions — a NULL session against a tenant-scoped resource is
-- denied with `no_tenant_binding`.
ALTER TABLE starter_auth_users_sessions ADD COLUMN tenant_id TEXT;

CREATE INDEX IF NOT EXISTS idx_sessions_tenant
    ON starter_auth_users_sessions (tenant_id);

CREATE TRIGGER IF NOT EXISTS trg_sessions_user_tenant_immutable
BEFORE UPDATE OF user_id, tenant_id ON starter_auth_users_sessions
FOR EACH ROW
WHEN OLD.user_id IS NOT NEW.user_id
  OR (OLD.tenant_id IS NOT NULL AND OLD.tenant_id IS NOT NEW.tenant_id)
BEGIN
    SELECT RAISE(ABORT, 'starter_auth_users_sessions: (user_id, tenant_id) are immutable');
END;
