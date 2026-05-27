-- Promote `'system'` from a reserved-slug pseudo-tenant into a real
-- tenant row. See migrations_postgres/.../0007_system_tenant.sql for
-- the design notes; this is the SQLite mirror.
--
-- SQLite does not support `ALTER TABLE ... DROP CONSTRAINT`, so we
-- have to rebuild the table to drop the `slug = 'system'`
-- reservation from the CHECK list.
--
-- The rebuild also forces us to drop+recreate the memberships table
-- because of the foreign key. Foreign keys are off by default in
-- sqlx migrations, but we restore membership rows explicitly via a
-- temporary backup so the operation is safe regardless.

CREATE TEMPORARY TABLE _mem_backup AS
    SELECT * FROM starter_auth_users_memberships;

DROP INDEX IF EXISTS idx_memberships_user;
DROP TABLE starter_auth_users_memberships;

CREATE TABLE starter_auth_users_tenants_new (
    id                  TEXT PRIMARY KEY,
    slug                TEXT NOT NULL UNIQUE,
    display_name        TEXT NOT NULL,
    audit_allow_sample  INTEGER,
    created_at          TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (
        slug NOT IN (
            'admin','api','auth','v1','v2','static','health',
            'metrics','openapi','extensions','mcp','tools',
            'default'
        )
        AND slug NOT GLOB '[0-9]*'
    )
);

INSERT INTO starter_auth_users_tenants_new (id, slug, display_name, audit_allow_sample, created_at)
    SELECT id, slug, display_name, audit_allow_sample, created_at
      FROM starter_auth_users_tenants;

DROP TABLE starter_auth_users_tenants;
ALTER TABLE starter_auth_users_tenants_new RENAME TO starter_auth_users_tenants;

CREATE TABLE starter_auth_users_memberships (
    tenant_id   TEXT NOT NULL REFERENCES starter_auth_users_tenants(id) ON DELETE CASCADE,
    user_id     TEXT NOT NULL REFERENCES starter_auth_users_users(id) ON DELETE CASCADE,
    role        TEXT NOT NULL CHECK (role IN ('reader','writer','admin')),
    created_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (tenant_id, user_id)
);

INSERT INTO starter_auth_users_memberships (tenant_id, user_id, role, created_at)
    SELECT tenant_id, user_id, role, created_at FROM _mem_backup;

DROP TABLE _mem_backup;

CREATE INDEX IF NOT EXISTS idx_memberships_user
    ON starter_auth_users_memberships (user_id);

INSERT OR IGNORE INTO starter_auth_users_tenants (id, slug, display_name)
    VALUES ('system', 'system', 'System');

INSERT OR IGNORE INTO starter_auth_users_memberships (tenant_id, user_id, role)
    SELECT 'system', id, 'admin'
      FROM starter_auth_users_users
     WHERE role = 'admin';
