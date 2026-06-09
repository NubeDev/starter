-- Tenant isolation foundation: a runtime role that cannot bypass RLS, and the
-- first tenant-scoped table guarded by a policy reading the per-transaction
-- `app.tenant_id` GUC. Every tenant-scoped table created later follows this same
-- shape (FORCE ROW LEVEL SECURITY + a USING/WITH CHECK policy on app.tenant_id).

-- The role nexus-api connects as in production. It is NOT a superuser, NOT the
-- table owner, and crucially does NOT have BYPASSRLS — owners and BYPASSRLS
-- roles skip policies, which would silently defeat tenancy. Created idempotently
-- so the migration is safe to re-run against a fresh database.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'nexus_runtime') THEN
        CREATE ROLE nexus_runtime NOLOGIN;
    END IF;
END
$$;

-- Datasources: one connection config per row, owned by a tenant. The secret is
-- stored only as ciphertext (envelope-encrypted); the plaintext never lands
-- here. `id` is the immutable handle grants and panel refs key on.
CREATE TABLE nexus_datasources (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id     text NOT NULL,
    name          text NOT NULL,
    kind          text NOT NULL,
    host          text NOT NULL,
    port          integer NOT NULL,
    database      text NOT NULL,
    db_user       text NOT NULL,
    secret_cipher bytea NOT NULL,
    secret_nonce  bytea NOT NULL,
    key_version   integer NOT NULL DEFAULT 1,
    created_at    timestamptz NOT NULL DEFAULT now()
);

-- Enable RLS and FORCE it so even the table owner is subject to the policy —
-- isolation must not depend on which role happens to run the query.
ALTER TABLE nexus_datasources ENABLE ROW LEVEL SECURITY;
ALTER TABLE nexus_datasources FORCE ROW LEVEL SECURITY;

-- A row is visible/writable only when its tenant matches the GUC the request
-- middleware sets. `current_setting(..., true)` returns NULL (not an error) when
-- the GUC is unset, so a query that forgot to bind a tenant sees zero rows
-- rather than everything.
CREATE POLICY nexus_datasources_tenant_isolation ON nexus_datasources
    USING (tenant_id = current_setting('app.tenant_id', true))
    WITH CHECK (tenant_id = current_setting('app.tenant_id', true));

-- The runtime role may read/write rows (subject to the policy above) but owns
-- nothing and cannot alter the schema or the policy.
GRANT SELECT, INSERT, UPDATE, DELETE ON nexus_datasources TO nexus_runtime;
