-- rubix-owned `rubix_users` table backing
-- `PgUserAdminStore`. Distinct from the auth-side `users` table
-- owned by `starter-auth-users`: the rubix-side row is the
-- verb-surface user (id + email + role + disabled + prefs +
-- tenant assignment) the
-- `rubix.user.{create,list,disable,enable,role.set,prefs.set,tenant.assign}`
-- verbs mutate. Unifying the two surfaces is intentionally out
-- of scope; see `rubix/crates/rubix-tools/src/user/store.rs`
-- module docs and the `rubix.tenant.*` sibling slice.
--
-- Schema mirrors `rubix_spi::user::UserRow` byte-exact:
-- - `user_id` is the operator-visible stable id (TEXT, not UUID,
--   so bundled seeds and operator-typed ids stay readable).
-- - `email` carries a `UNIQUE` constraint so the Pg impl's
--   `create` does not have to walk the table the way the
--   in-memory impl does. The trait contract enumerates this as
--   the only uniqueness key.
-- - `role` is TEXT (`reader` / `writer` / `admin`); the wire
--   shape is a string and `starter_spi::auth::Role` enum is
--   serde-`rename_all = "lowercase"`. A CHECK constraint would
--   add coverage but couples this migration to the enum
--   variant set; deferred until we add a `role.add` verb that
--   could introduce a typo.
-- - `disabled_at_ms BIGINT NULL` mirrors the row's
--   `Option<i64>`. NULL = enabled.
-- - `prefs_json JSONB NULL` mirrors `Option<serde_json::Value>`.
--   JSONB so the warehouse-explorer + future per-pref indexes
--   can use jsonb operators; the wire shape stays
--   `serde_json::Value`.
-- - `tenant_id TEXT NULL REFERENCES rubix_tenants(tenant_id) ON DELETE RESTRICT`.
--   Real FK + restrict for defense in depth: the
--   `rubix.tenant.delete` verb already refuses-if-referenced
--   at the verb layer, the DB constraint catches any path
--   that bypasses the verb (direct undo replay, an operator
--   running raw SQL, a future cross-actor race). Choice
--   recorded in the `2026-05-28-pg-rubix-tenant-store.md`
--   session's follow-ups.
--
-- No `created_at_ms` / `updated_at_ms` on the row: the snapshot
-- Reversible carries the full row, and the §3.1 echo rule
-- reduces to the six fields above. If `user.list` ever surfaces
-- "last touched" to an operator, the columns + `Option<i64>`
-- row fields land then.

CREATE TABLE IF NOT EXISTS rubix_users (
    user_id        TEXT PRIMARY KEY,
    email          TEXT NOT NULL UNIQUE,
    role           TEXT NOT NULL,
    disabled_at_ms BIGINT,
    prefs_json     JSONB,
    tenant_id      TEXT
        REFERENCES rubix_tenants (tenant_id)
        ON DELETE RESTRICT
);

-- Index the FK column so the `tenant.delete` refuse-if-referenced
-- pre-check (which runs `SELECT 1 FROM rubix_users WHERE tenant_id = $1`
-- under the hood) and the `user.list?tenant_id=` filter both
-- stay sub-millisecond as the user table grows.
CREATE INDEX IF NOT EXISTS rubix_users_tenant_id_idx
    ON rubix_users (tenant_id)
    WHERE tenant_id IS NOT NULL;
