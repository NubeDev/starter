-- rubix-owned `rubix_tenants` table backing
-- `PgRubixTenantStore`. Distinct from the auth-side `tenants`
-- table owned by `starter-auth-users`: the rubix-side row is the
-- verb-surface tenant (id + name + locale) the
-- `rubix.tenant.{create,update,list,delete}` verbs mutate and the
-- `rubix.user.tenant.assign` verb validates against. Unifying the
-- two surfaces is intentionally out of scope; see
-- `rubix/crates/rubix-tools/src/tenant/store.rs` module docs.
--
-- Schema mirrors `rubix_spi::tenant::TenantRow` byte-exact:
-- - `tenant_id` is the operator-visible stable id (TEXT, not UUID,
--   so bundled rows like the `"system"` seed stay readable).
-- - `name` is a separate operator-visible unique key. The trait
--   contract requires uniqueness on both id AND name; enforced
--   here with a `UNIQUE` constraint so the Pg impl does not have
--   to do the second walk the in-memory impl does.
-- - `locale` defaults to `'en'` so newly seeded rows do not blow
--   up if a future caller forgets the column.
--
-- No timestamps: the §3.1 echo rule for this kind reduces to the
-- three fields above (snapshot Reversible, see
-- `TenantReversible`).

CREATE TABLE IF NOT EXISTS rubix_tenants (
    tenant_id TEXT PRIMARY KEY,
    name      TEXT NOT NULL UNIQUE,
    locale    TEXT NOT NULL DEFAULT 'en'
);
