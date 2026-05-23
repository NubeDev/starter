-- Phase 7b — teams + team_members (SCOPE-EXT.md R13).
--
-- A team is a named, tenant-scoped collection of users. Membership
-- is read-mostly. The slug is the rule-stable identity (rules say
-- `principal.teams contains "hvac-ops"`); display_name is the
-- human-readable label and can be renamed without invalidating
-- rules. The slug is therefore immutable after create — a BEFORE
-- UPDATE trigger refuses any change to slug or tenant_id.
CREATE TABLE IF NOT EXISTS starter_auth_users_teams (
    id           TEXT PRIMARY KEY,
    tenant_id    TEXT NOT NULL REFERENCES starter_auth_users_tenants(id) ON DELETE CASCADE,
    slug         TEXT NOT NULL,
    display_name TEXT NOT NULL,
    created_at   TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (tenant_id, slug)
);

CREATE INDEX IF NOT EXISTS idx_teams_tenant
    ON starter_auth_users_teams (tenant_id);

-- slug is rule-stable — once written, it never changes. tenant_id
-- is immutable for the same reason (a team can't be moved
-- cross-tenant; admins must delete + re-create). display_name
-- stays mutable for UI-only renames.
CREATE TRIGGER IF NOT EXISTS trg_teams_slug_tenant_immutable
BEFORE UPDATE OF slug, tenant_id ON starter_auth_users_teams
FOR EACH ROW
WHEN OLD.slug IS NOT NEW.slug OR OLD.tenant_id IS NOT NEW.tenant_id
BEGIN
    SELECT RAISE(ABORT, 'starter_auth_users_teams: (tenant_id, slug) are immutable');
END;

CREATE TABLE IF NOT EXISTS starter_auth_users_team_members (
    team_id    TEXT NOT NULL REFERENCES starter_auth_users_teams(id) ON DELETE CASCADE,
    user_id    TEXT NOT NULL REFERENCES starter_auth_users_users(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (team_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_team_members_user
    ON starter_auth_users_team_members (user_id);
