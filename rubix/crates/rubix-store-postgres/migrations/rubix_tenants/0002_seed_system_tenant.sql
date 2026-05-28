-- Seed the bundled `"system"` tenant the registry boot path
-- expected from the in-memory `InMemoryTenantStore::seeded(...)`
-- shape. Without this row a fresh Pg-backed boot would surface
-- an empty tenant list and the bundled SDUI pages (whose
-- `tenant_id` is `rubix_spi::dashboard::BUNDLED_TENANT = "system"`)
-- would resolve against nothing.
--
-- `ON CONFLICT DO NOTHING` so the migration is idempotent and so
-- an operator who renamed the system tenant manually is not
-- overwritten by a routine re-deploy. The seed name `'System'`
-- is the same one the in-memory boot path uses.

INSERT INTO rubix_tenants (tenant_id, name, locale) VALUES
    ('system', 'System', 'en')
ON CONFLICT (tenant_id) DO NOTHING;
