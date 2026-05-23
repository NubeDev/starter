-- starter-auth-users tenants + memberships. Postgres mirror of
-- 0005_tenants.sql in migrations/starter_auth_users/.
--
-- Translation notes (vs sqlite):
--   sqlite TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
--   →     TIMESTAMPTZ NOT NULL DEFAULT NOW()
--   sqlite `slug NOT GLOB '[0-9]*'`
--   →     POSIX regex `slug !~ '^[0-9]'`
--   sqlite `CREATE TRIGGER ... BEGIN SELECT RAISE(ABORT, msg); END;`
--   →     Postgres `CREATE FUNCTION ... RAISE EXCEPTION ... USING
--          ERRCODE = 'check_violation'; CREATE TRIGGER ... EXECUTE
--          FUNCTION ...;` so callers can detect SQLSTATE 23514.
--
-- The Postgres migration set is fresh; `tenant_id` is already
-- baked into the sessions / tokens tables (0002, 0003) so this
-- file only adds the new relations and the immutability triggers.
-- See docs/design/auth/README.md for the multi-tenant contract.
--
-- `slug` is the URL-facing tenant identifier. A reserved-slug list
-- is enforced at INSERT via a CHECK constraint so writes that race
-- past the application-level check still fail loudly.
CREATE TABLE IF NOT EXISTS starter_auth_users_tenants (
    id                  TEXT PRIMARY KEY,
    slug                TEXT NOT NULL UNIQUE,
    display_name        TEXT NOT NULL,
    -- Per-tenant override for the audit-log allow-sample rate.
    -- NULL = use the global STARTER_AUTHZ_DECISION_ALLOW_SAMPLE.
    audit_allow_sample  INTEGER,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (
        slug NOT IN (
            'admin','api','auth','v1','v2','static','health',
            'metrics','openapi','extensions','mcp','tools',
            'default','system'
        )
        AND slug !~ '^[0-9]'
    )
);

CREATE TABLE IF NOT EXISTS starter_auth_users_memberships (
    tenant_id   TEXT NOT NULL REFERENCES starter_auth_users_tenants(id) ON DELETE CASCADE,
    user_id     TEXT NOT NULL REFERENCES starter_auth_users_users(id) ON DELETE CASCADE,
    role        TEXT NOT NULL CHECK (role IN ('reader','writer','admin')),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (tenant_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_memberships_user
    ON starter_auth_users_memberships (user_id);

-- (user_id, tenant_id) immutability on tokens. Once a token row
-- exists those columns may not be UPDATEd — a buggy migration
-- cannot silently move a token to a different tenant.
CREATE OR REPLACE FUNCTION starter_auth_users_tokens_user_tenant_immutable()
RETURNS trigger AS $$
BEGIN
    IF NEW.user_id IS DISTINCT FROM OLD.user_id
       OR NEW.tenant_id IS DISTINCT FROM OLD.tenant_id THEN
        RAISE EXCEPTION
            'starter_auth_users_tokens: (user_id, tenant_id) are immutable'
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_tokens_user_tenant_immutable
    ON starter_auth_users_tokens;
CREATE TRIGGER trg_tokens_user_tenant_immutable
BEFORE UPDATE ON starter_auth_users_tokens
FOR EACH ROW
EXECUTE FUNCTION starter_auth_users_tokens_user_tenant_immutable();

-- (user_id, tenant_id) immutability on sessions. tenant_id may
-- transition from NULL → set exactly once (login binds a
-- pre-Phase-7 session to a tenant); any other change is refused.
CREATE OR REPLACE FUNCTION starter_auth_users_sessions_user_tenant_immutable()
RETURNS trigger AS $$
BEGIN
    IF NEW.user_id IS DISTINCT FROM OLD.user_id
       OR (OLD.tenant_id IS NOT NULL
           AND NEW.tenant_id IS DISTINCT FROM OLD.tenant_id) THEN
        RAISE EXCEPTION
            'starter_auth_users_sessions: (user_id, tenant_id) are immutable'
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_sessions_user_tenant_immutable
    ON starter_auth_users_sessions;
CREATE TRIGGER trg_sessions_user_tenant_immutable
BEFORE UPDATE ON starter_auth_users_sessions
FOR EACH ROW
EXECUTE FUNCTION starter_auth_users_sessions_user_tenant_immutable();
