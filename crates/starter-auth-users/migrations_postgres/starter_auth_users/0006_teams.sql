-- starter-auth-users teams + team_members. Postgres mirror of
-- 0006_teams.sql in migrations/starter_auth_users/.
--
-- A team is a named, tenant-scoped collection of users. The slug
-- is the rule-stable identity (`principal.teams contains "hvac-ops"`);
-- display_name is human-readable and may be renamed. Slug + tenant
-- are immutable after create — enforced by a BEFORE UPDATE trigger
-- that raises SQLSTATE 23514 (check_violation) on any change.
-- See docs/design/auth/README.md for the rule-stability contract.
CREATE TABLE IF NOT EXISTS starter_auth_users_teams (
    id           TEXT PRIMARY KEY,
    tenant_id    TEXT NOT NULL REFERENCES starter_auth_users_tenants(id) ON DELETE CASCADE,
    slug         TEXT NOT NULL,
    display_name TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_id, slug)
);

CREATE INDEX IF NOT EXISTS idx_teams_tenant
    ON starter_auth_users_teams (tenant_id);

CREATE OR REPLACE FUNCTION starter_auth_users_teams_slug_tenant_immutable()
RETURNS trigger AS $$
BEGIN
    IF NEW.slug IS DISTINCT FROM OLD.slug
       OR NEW.tenant_id IS DISTINCT FROM OLD.tenant_id THEN
        RAISE EXCEPTION
            'starter_auth_users_teams: (tenant_id, slug) are immutable'
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_teams_slug_tenant_immutable
    ON starter_auth_users_teams;
CREATE TRIGGER trg_teams_slug_tenant_immutable
BEFORE UPDATE ON starter_auth_users_teams
FOR EACH ROW
EXECUTE FUNCTION starter_auth_users_teams_slug_tenant_immutable();

CREATE TABLE IF NOT EXISTS starter_auth_users_team_members (
    team_id    TEXT NOT NULL REFERENCES starter_auth_users_teams(id) ON DELETE CASCADE,
    user_id    TEXT NOT NULL REFERENCES starter_auth_users_users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (team_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_team_members_user
    ON starter_auth_users_team_members (user_id);
