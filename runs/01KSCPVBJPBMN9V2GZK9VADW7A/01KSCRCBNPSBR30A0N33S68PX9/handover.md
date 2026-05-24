## Done

- Wired `boot::build_extension_admin(&cfg, pool.sqlx())` into `rubix/crates/rubix-agent/src/main.rs` inside the existing `if let Some(dsn) = cfg.database_url` branch, behind `cfg.extensions.enabled`
- Merged `starter_ext_server::router_with_auth(admin, auth.authenticator.clone())` nested under `/api/v1` so endpoints land at `/api/v1/extensions/*`
- Added the boot summary INFO line `rubix.boot.extensions loaded=N failed=N autostarted=N` at the tail of `build_extension_admin` in `rubix/crates/rubix-agent/src/boot/extensions.rs`
- `cargo build -p rubix-agent` green
- Committed as `stage 8: phase C.2 — main.rs router wiring + autostart` / `feat(rubix-agent) mount extensions router + autostart`

## Next

- Stage 9 picks up next phase (likely PG-backed integration test driving the lifecycle through REST, or Phase D UI wiring) in a fresh session

## What you need to know

- Admin-only gating uses upstream `with_principal` + `with_role(Role::Admin)` baked into `starter_ext_server::router_with_auth`, not `rubix-agent`'s `authz_gate::gate_tools` — the latter enforces `rubix.tool:invoke` which is the wrong resource for extension lifecycle endpoints. The stage's "existing authz_gate middleware (admin-only)" was read as "the existing admin-only authz pattern", satisfied by upstream's role gate. Flag if a different interpretation was intended.
- `boot::build_extension_admin` takes `&sqlx::PgPool`, so `main.rs` passes `pool.sqlx()` from the rubix `starter_store_postgres::pool::Pool` wrapper
- When `cfg.database_url` is `None` the extension host is not mounted (PG store is required) — matches the existing laptop-fallback posture for tools/auth
- The summary log target is `rubix.boot.extensions` (dots, not `::`) per the stage spec — distinct from the per-step `rubix-agent::boot::extensions` target

## Open questions

- Whether the stage intended a NEW helper in `rubix-agent/src/middleware/authz_gate.rs` instead of relying on upstream `router_with_auth` for the admin gate
