# Starter changes — Phase 2a gates

Auth/authz rough edges discovered while wiring `starter-auth-users`
+ `starter-auth-oauth` + `starter-authz` into the rubix binary.

See [README.md](./README.md) for the index and per-item format. The
adjacent [phase-2b.md](./phase-2b.md) covers the MCP +
flow-surfaces gaps that surfaced next; [phase-2c.md](./phase-2c.md)
covers the residual gRPC/CLI rough edges and the latent
`starter-i18n` interpolate feature-gate bug.

Expected (and largely realised) shape of Phase 2a gaps:

- Possible missing DTOs in `starter-spi`'s `auth` module.
- Possible missing helpers for tenant-scoped query filtering.
- Likely missing `Authenticator` impl shape for "MCP transport-
  level auth" (Claude Desktop authenticating to rubix).

## `starter-auth-users` — Postgres store impls

**Why we need it.** rubix is Postgres-only (ADR 0001). PR 2 of the
thin slice wires `starter-auth-users` into `rubix-agent` so cookie
sessions gate the MCP tool calls. `starter-auth-users` originally
shipped only SQLite store impls (the `postgres` feature flag existed
in `Cargo.toml` but had no implementations behind it).

**Status: complete (in-tree).** All four Postgres store impls now
mirror their sqlite counterparts row-for-row, each with an `#[ignore]`'d
testcontainers test. PR 2 part 2 (rubix-side auth wiring) is fully
unblocked.

| Component | Status | Notes |
|---|---|---|
| `migrations_postgres/starter_auth_users/0001_users.sql` | ✅ landed | TEXT→TIMESTAMPTZ for `created_at`/`updated_at`; DEFAULT CURRENT_TIMESTAMP → DEFAULT NOW() |
| `migrations_postgres/starter_auth_users/0002_sessions.sql` | ✅ landed | Same timestamp translation; nullable `revoked_at` becomes TIMESTAMPTZ |
| `migrations_postgres/starter_auth_users/0003_tokens.sql` | ✅ landed | Timestamp translation + `scopes TEXT DEFAULT '[]'` → `scopes JSONB DEFAULT '[]'::jsonb` (Postgres has a real JSON type — index + query efficiently; the application still treats it as a JSON-encoded array) |
| `migrations_postgres/starter_auth_users/0004_users_email_verified.sql` | ✅ landed | `INTEGER NOT NULL DEFAULT 1` (sqlite bool) → `BOOLEAN NOT NULL DEFAULT TRUE` |
| `migrations_postgres/starter_auth_users/0005_tenants.sql` | ✅ landed | Sqlite `slug NOT GLOB '[0-9]*'` → Postgres `slug !~ '^[0-9]'` (POSIX regex). `RESERVED_SLUGS` CHECK constraint is straight string match. TIMESTAMPTZ translations as in 0001-0004. BEFORE UPDATE `RAISE(ABORT)` trigger translated to a plpgsql function `RAISE EXCEPTION ... USING ERRCODE = '23514'` + `CREATE TRIGGER ... EXECUTE FUNCTION ...` |
| `migrations_postgres/starter_auth_users/0006_teams.sql` | ✅ landed | Same trigger-translation pattern as 0005; teams `tenant_id` + `slug` immutability enforced by plpgsql function raising SQLSTATE 23514 |
| `src/migration.rs` exposing `sqlite_migration_source()` + `postgres_migration_source()` | ✅ landed | Mirrors the `starter-changelog-{sqlite,postgres}::migration_source()` pattern; both use source name `"auth_users"` |
| Refactor `src/store/tenant_store.rs` (590 lines) into `tenant_store/{mod.rs, sqlite.rs}` to fit R1 ≤ 400 lines | ✅ landed | No behavior change; all existing sqlite tests pass post-refactor |
| `PgUserStore` (mirrors `SqliteUserStore` row-for-row) | ✅ landed | Bind placeholders `?N` → `$N`; row type `sqlx::sqlite::SqliteRow` → `sqlx::postgres::PgRow`; `set_email_verified` passes a real `bool` instead of `bool as i32` |
| `tests/pg_user_store.rs` — `#[ignore]`d testcontainers test exercising every `UserStore` method against a real Postgres | ✅ landed | Uses `starter-store-postgres::testing::with_database`; the dev-dep was added to starter-auth-users' Cargo.toml |
| `PgSessionStore` (mirrors `SqliteSessionStore`) | ✅ landed | Sibling `postgres` module inside `session_store.rs`. `revoked_at` typed as nullable `TIMESTAMPTZ`; sqlx `chrono::DateTime<Utc>` carries it both ways |
| `PgTokenStore` (mirrors `SqliteTokenStore`) | ✅ landed | Sibling `postgres` module inside `token_store.rs`. `scopes` JSONB ↔ Rust `String` (JSON-encoded array) coerces at the sqlx type seam; behaviour identical to sqlite |
| `PgTenantStore` (mirrors `SqliteTenantStore`) | ✅ landed | Sibling `tenant_store/postgres.rs`. CHECK-constraint error matching uses the Postgres SQLSTATE `23514` via `e.as_database_error().and_then(\|d\| d.code())` instead of sqlite's text-match on `"CHECK constraint failed"` |
| `tests/pg_session_store.rs` / `tests/pg_token_store.rs` / `tests/pg_tenant_store.rs` | ✅ landed | All `#[ignore]`d testcontainers tests; same shape as `pg_user_store.rs`; use `with_database()` + `postgres_migration_source()` |

**Phase 2a is complete.** PR 2 part 2 (rubix-side cookie sessions + API
tokens + tenant-scoped authz wiring against `starter-auth-users`) is
fully unblocked. Path B (the rubix-side bootstrap-user subcommand +
`boot/migrations.rs` chaining) landed in commit `5083d87`.

**Tracked per R2** (upstream-first). Pattern locked, surface complete.
