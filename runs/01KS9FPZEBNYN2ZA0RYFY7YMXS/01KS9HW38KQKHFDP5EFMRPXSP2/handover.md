## Done

- starter-spi: `Principal.teams: Vec<String>` (serde-default empty so it's strictly additive).
- starter-auth-users migration `0006_teams.sql`: `starter_auth_users_teams` + `starter_auth_users_team_members`, BEFORE UPDATE trigger making `(slug, tenant_id)` immutable; `display_name` mutable.
- `TenantStore` gains `create_team` / `delete_team` / `get_team` / `list_teams` / `add_team_member` / `remove_team_member` / `team_slugs_for_user`; SQLite impl JOINs through `teams` filtered by `tenant_id` so a leaked membership row cannot surface a slug from another tenant (R13 tenant-scope).
- Authenticator population: `session::verify_session_with_teams[_and_extras]`, `token::verify_with_teams`, `AuthAuthenticator::with_tenants(...)` wires the lookup once and runs it on every verify. Super-admin sentinel `"*"` short-circuits to empty teams.
- starter-authz: condition grammar gains `path 'contains' value`. `Expr::eval` keeps `bool` for back-compat; new `Expr::try_eval` returns `Result<bool, EvalError>` with `EvalError::ContainsLhsNotArray { path, actual_type }`. Engine calls `try_eval` and maps the typed error to `Decision::deny_by("condition_invalid", rule.id)` — loud failure per R13. Context exposes `principal.teams/.subject/.role/.tenant`.
- Admin REST: `POST/GET /v1/tenants/{id}/teams`, `DELETE /v1/tenants/{id}/teams/{team_id}`, `POST .../members`, `DELETE .../members/{user_id}`.
- `examples/authz-demo`: `admin::grant_team(...)` helper writes one tenant-scoped rule covering every team member.
- Tests (all passing): `crates/starter-authz/tests/team_rules.rs` (5) — team-grant-coverage, team-membership-remove-takes-effect, team-rules-tenant-scoped, engine-compile-error-when-contains-LHS-not-array, contains-with-missing-lhs-is-silent-false-not-error. `crates/starter-auth-users/tests/teams_smoke.rs` (4) — store-side coverage including the slug/tenant immutability trigger verified via raw UPDATE.
- 30 workspace crates mass-updated with `teams: Vec::new()` on every `Principal { … }` literal so the new field is non-breaking.
- Committed as `3d2257c` on `codeless/authz-phase-7`.

## Next

- Stage 3 (slice 7c — Decision audit log): `DecisionSink` trait + `NoopDecisionSink` + `DbDecisionSink` writing to `starter_authz_decisions`, 100% deny retention + 1-in-N allow sampling, `spawn_retention(...)`, `GET /v1/authz/decisions`. Smoke tests per SCOPE-EXT.md.

## What you need to know

- `Principal.teams` defaults to `[]` and is `#[serde(skip_serializing_if = "Vec::is_empty")]`, so wire formats only carry it for principals that actually have teams — Phase 1–6 payloads round-trip unchanged.
- The `contains` LHS-not-array path is a hard `Deny { reason: "condition_invalid", matched_rule }`. **Missing** LHS (`principal.teams` absent entirely) is still silent-false, matching R8's "additive by default" shape — see `contains_with_missing_lhs_is_silent_false_not_error`. The discriminator is `None` vs `Some(non-Array)`.
- `add_team_member` is idempotent on unique-violation (re-adding a member returns Ok) — the admin REST returns `201 Created` regardless; callers needing strict "was-already-member" semantics should query first.
- `team_slugs_for_user` JOINs `team_members → teams WHERE teams.tenant_id = ?` — this is what enforces R13 tenant-scoping at the store layer even before R11's predicate fires.
- The auth-users crate is sqlite-only today (mirroring stage 1). Postgres trigger SQL is not shipped; when a `PgTenantStore` lands, mirror `trg_teams_slug_tenant_immutable` as a `CREATE OR REPLACE FUNCTION ... RAISE EXCEPTION 'immutable' ... CREATE TRIGGER ... BEFORE UPDATE`.
- `examples/authz-demo` exposes `admin::grant_team` but the CLI subcommand wiring (a `grant-team` arg parallel to `grant`) is intentionally left for slice 7d when the demo is also reshaped for the per-entry-permission story. The helper is what the slice 7b smoke wants and what slice 7d will reuse.
- Stage 1 (slice 7a) gaps documented in the previous handover (OAuth interstitial, R12 owner-immutability triggers on built-in tables, `STARTER_AUTHZ_PHASE_7_DEFAULT_TENANT` backfill, multi-tenant/OAuth/Postgres-trigger smokes) are still open. They were not in scope for this stage; the REVIEW gate after slice 7a remains the right place to close them.
- Workspace-wide `cargo build --workspace` requires excluding `starter-blob-s3` and `starter-blob-garage` on this toolchain (rustc 1.90.0 vs `aws-types@1.3.16` needing 1.91.1) — pre-existing on master, not from this stage.
- `starter-grpc`'s `tools_service` test fails to compile on master (`starter_grpc::testing` missing) — pre-existing, not from this stage; verified by `git stash`.

## Open questions

- Whether tenant slugs should be added as `principal.teams` automatically (so "every member of tenant X" is one rule). Spec doesn't ask for it; deferred to a separate ADR if real rules need it.
- `add_team_member` idempotency: should it 200 vs 201 on the already-member case? Current behaviour always returns 201; not specified.
