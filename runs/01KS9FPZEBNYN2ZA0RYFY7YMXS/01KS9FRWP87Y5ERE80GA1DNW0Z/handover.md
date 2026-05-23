## Done

- SPI: `Principal.tenant_id`, `ResourceRef.tenant`, `ResourceSpec.tenant_scoped` + `from_static_tenant_scoped` + `Principal::is_super_admin` for the `"*"` sentinel.
- starter-authz engine: cross-tenant predicate runs BEFORE role/condition with typed deny reasons `cross_tenant` and `no_tenant_binding`; rule.tenant_id filters in the match loop; super-admin `"*"` bypasses both checks; ctx exposes `tenant` and `object.tenant`. `StoredRule.tenant_id` + `Rule.tenant_id` + sqlite & postgres migrations `0003_authz_rules_tenant.sql`.
- starter-auth-users migration `0005_tenants.sql`: tenants + memberships tables; reserved-slug CHECK constraint (`admin`,`api`,`auth`,`v1`,`v2`,`static`,`health`,`metrics`,`openapi`,`extensions`,`mcp`,`tools`,`default`,`system` + all-digits); `sessions.tenant_id` + `tokens.tenant_id` columns; BEFORE UPDATE triggers refusing changes to `(user_id, tenant_id)` on both tables (`RAISE(ABORT,...)`).
- `TenantStore` trait + `SqliteTenantStore` with `remove_member` cascading to `token revoke_for_membership` inside one transaction (R12).
- `TokenStore::create` gains `tenant_id` arg; `SUPER_ADMIN_TENANT="*"` constant; `revoke_for_membership` added; session create/issue gains tenant; verify paths populate `Principal.tenant_id` from the row.
- `/v1/tenants` admin REST: POST/GET, GET/PATCH `/v1/tenants/{id}` (slug immutable, `audit_allow_sample` patchable), POST `/v1/tenants/{id}/members`, PATCH/DELETE `/v1/tenants/{id}/members/{user_id}`. Explicitly NO DELETE on the tenant itself (deferred to ADR-tenant-deletion).
- Smoke tests passing: `tenant_predicate.rs` (6 tests — cross-tenant-deny with role:*/resource:*/actions:[*] allow rule, no-tenant-binding, same-tenant pass-through, global-resource bypass, super-admin sentinel, tenant-scoped rule isolation); `tenants_smoke.rs` (5 tests — reserved-slug rejection incl. all-digits, token immutability trigger, membership-revoke cascade); `tenants_admin_routes.rs` (2 tests — create+list, reserved-slug→400).
- Mechanical Principal/ResourceRef literal updates across the workspace (60+ sites) so `cargo check --tests` is green for every crate touched: starter-spi, starter-authz, starter-auth-users, starter-auth-oauth, starter-auth-token, starter-server, starter-mcp, starter-grpc, starter-flow*, starter-clipboard*, starter-changelog*, starter-skills, starter-prefs, starter-ui-theme, starter-cli, starter-sdui-routes, starter-smoke-tests, starter-store-*, starter-authz-demo.
- Demo (`examples/authz-demo`) updated minimally so it still compiles: tokens issue with `tenant="*"`, grants write `tenant_id=None` (global rules).
- Committed as `1624372` on `codeless/authz-phase-7`.

## Next

- Continue stage 1 in a follow-up session to close the remaining 7a items (see Open questions) — they were left out of this commit because the disk had filled up mid-build and the scope is too large for one session. After they land, move to stage 2 (slice 7b — Teams).

## What you need to know

- Engine semantics: `tenant_scoped=false` (default) keeps pre-Phase-7 behaviour for every existing kind — the predicate is skipped entirely. Consumers opt in by passing `from_static_tenant_scoped` (or constructing a `ResourceSpec { tenant_scoped: true, .. }`) at registration time.
- Sentinel: `Principal.tenant_id == Some("*")` bypasses the cross-tenant predicate AND matches every tenant-scoped rule. The auth layer is responsible for only minting this binding for users with global Admin role; `token::issue` accepts `"*"` for any user, on the assumption that callers (the admin token route) gate on role first.
- Pre-Phase-7 token rows backfill to `tenant_id='*'` (documented in the migration). Operators wanting tenant-scoped existing tokens must revoke+reissue.
- Trigger semantics: SQLite uses `RAISE(ABORT, ...)` in `BEFORE UPDATE` triggers. Postgres-equivalent triggers for the tenant tables in `starter-auth-users` are NOT shipped (the crate is sqlite-only today — its Cargo.toml has a `postgres` feature gate but no impl exists). When a Postgres TenantStore lands, mirror the trigger as `CREATE OR REPLACE FUNCTION ... RAISE EXCEPTION 'immutable' ... CREATE TRIGGER ... BEFORE UPDATE`.
- The TenantStore::patch_tenant SQL uses a CASE-WHEN-on-bind-param trick to thread `Option<Option<i32>>` through sqlx without dynamic SQL. Read it twice before changing.
- `tower::ServiceExt::oneshot` works with `Router::<()>::new()...with_state(tenants)` — `tenants_router::<()>(tenants)` is the typing the test uses.
- Disk space on the build host hit 100% mid-session; one sibling worktree's target/ (26G) had to be deleted to keep going. Future sessions should `cargo clean` before re-running the full workspace test.

## Open questions

- OAuth callback tenant resolution: `?tenant=<slug>` validated against memberships, single-membership auto-selects, multi-membership renders a hand-rolled HTML `POST /v1/auth/oauth/select-tenant` interstitial. None of this shipped — `starter-auth-oauth` compiles unchanged because session::issue defaults the new tenant arg to None, but the OAuth flow still mints tenantless sessions. Land before declaring stage 1 done.
- `(tenant_id, owner_id)` immutability triggers on the built-in tenant-scoped tables listed in the scope (reports, flows, pages, marts, sandboxes, sessions). None landed; each crate's migrations need their own `0NNN_tenant_owner_immutable.sql`. Sessions (in starter-auth-users) is the only one already done.
- `STARTER_AUTHZ_PHASE_7_DEFAULT_TENANT` backfill migration with the "refuses to run if unset, prints manual-mapping caveat" gate per R12. Not shipped.
- `examples/authz-demo`: seed two tenants and exercise the cross-tenant deny path end-to-end (currently the demo just compiles).
- Remaining required smoke tests not covered by the engine-level tests: `multi-tenant-session-binding`, `oauth-callback-resolves-tenant-via-query-param`, `oauth-callback-renders-interstitial-on-multi-membership`, `immutability-trigger-rejects-update` for Postgres (sqlite is done).
- Stage 1's REVIEW gate (the "cross-tenant data leak is the most expensive bug" gate) is intentionally still open — it should fire after the OAuth + R12-trigger work lands, not after this partial slice.
