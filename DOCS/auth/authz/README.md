# Authz

Policy-based authorization for `starter` binaries. Sits **after** the
authenticator (`starter-auth-users` / `starter-auth-token` /
`starter-auth-oauth`) and decides whether a verified `Principal` may
perform `<action>` on `<resource>`. RBAC + ownership + attribute
conditions, with both file-backed and DB-backed engines.

For the full design rationale and hard rules see
[`SCOPE.md`](./SCOPE.md). The implementation lives in the
[`starter-authz`](../../../crates/starter-authz/) crate.

## Quick start

```toml
# Cargo.toml of your binary
[dependencies]
starter-authz = { workspace = true, features = ["sqlite"] }
```

```rust,no_run
use std::sync::Arc;
use starter_authz::{
    authz_router, AuthzConfig, DbPolicyEngine, StaticRegistry,
    routes::AuthzRoutesState,
    store::{SqlitePolicyStore, AUTHZ_SQLITE_MIGRATOR},
};
use starter_spi::authz::{Ownership, ResourceSpec};
use starter_store_sqlite::{migrate, migrate::MigrationSource};

# async fn boot(pool: starter_store_sqlite::Pool) -> anyhow::Result<()> {
// 1. Apply migrations.
migrate(&pool)
    .with_source(MigrationSource {
        name: "starter_authz",
        migrator: &AUTHZ_SQLITE_MIGRATOR,
    })
    .run()
    .await?;

// 2. Register every resource kind the policy can reference (R4).
let registry = Arc::new(StaticRegistry::new());
registry.register_spec(ResourceSpec::from_static(
    "flows",
    &["read", "create", "update", "delete"],
    Ownership::Subject,
    "Flows",
    "User-authored automation flows.",
));

// 3. Build the engine (default policy on = ships Reader/Writer/Admin).
let store = Arc::new(SqlitePolicyStore::new(pool));
let engine = Arc::new(
    DbPolicyEngine::new(store, registry.clone(), true).await?,
);

// 4. Mount admin REST routes at /v1/authz/*.
let admin = authz_router::<()>(AuthzRoutesState {
    engine: engine.clone(),
    registry,
});
# Ok(()) }
```

Mount `engine` as an `Arc<dyn PolicyEngine>` extension on every
request *after* the authenticator, then gate routes with
`starter_authz::with_permission(router, "flows", "update")` or call
`check_or_deny(...)` inside handlers for row-level checks.

## Engines

| Engine | Feature | Source of truth | When to use |
|--------|---------|-----------------|-------------|
| `StaticRbacEngine` | (always on) | [`AuthzConfig`] (TOML or builder) | Static deployments; policy in version control. |
| `DbPolicyEngine`   | `sqlite` / `postgres` | `starter_authz_*` tables | Multi-tenant or admin-editable policy. |
| `NoopPolicyEngine` | (always on, in `starter-spi`) | n/a (always allow) | Tests, headless binaries where authz is intentionally disabled. |

The DB engine wraps a `StaticRbacEngine` internally and `reload()`s it
after every successful write through the admin REST routes — the hot
`check()` path is allocation-free and never blocks.

## Built-in roles

With `default_policy = true` the engine layers in three roles
(see [`defaults`](../../../crates/starter-authz/src/defaults.rs)):

| Role   | Reads | Writes (non-sensitive) | Writes (sensitive*) | Admin |
|--------|:-----:|:----------------------:|:-------------------:|:-----:|
| Reader |  ✓    |                        |                     |       |
| Writer |  ✓    |  ✓                     | own row only        |       |
| Admin  |  ✓    |  ✓                     |  ✓                  |  ✓    |

*Sensitive resources*: `users`, `sessions`, `tokens`, `secrets`,
`oauth_identities`. Writers may `update` their own row but not
`create` / `delete` (deny-overrides per R3).

## REST surface

All admin routes require `Principal.role == Admin`. Writes additionally
require the double-submit CSRF pair (`X-CSRF-Token` header ==
`starter_csrf` cookie).

| Method | Path                              | Purpose |
|--------|-----------------------------------|---------|
| GET    | `/v1/authz/rules`                 | List every rule (priority DESC, created_at ASC). |
| POST   | `/v1/authz/rules`                 | Create a rule. Cache reloads on success. |
| PUT    | `/v1/authz/rules/{id}`            | Replace a rule. |
| DELETE | `/v1/authz/rules/{id}`            | Remove a rule. |
| GET    | `/v1/authz/assignments`           | List subject→role bindings. |
| POST   | `/v1/authz/assignments`           | Create a binding. |
| DELETE | `/v1/authz/assignments/{id}`      | Remove a binding. |
| GET    | `/v1/authz/resources`             | Enumerate the registry for the admin UI. |
| POST   | `/v1/authz/check`                 | Dry-run a `(principal, action, resource)` tuple. |

The dry-run endpoint always agrees with what a real request would see
— this is enforced by the `dry-run-matches-real-check` smoke test.

## Decision reason codes

Denials carry stable codes (R9) so the HTTP layer can surface them as
`403 { "error": "<reason>" }` without leaking rule details:

| Code                  | Meaning |
|-----------------------|---------|
| `unknown_resource`    | Resource kind not in the registry. Default-deny. |
| `no_matching_rule`    | No rule allowed this tuple. Default-deny. |
| `explicit_deny`       | A rule with `effect = deny` matched. |
| `not_owner`           | A rule required `condition = "owner"` and the principal does not own the row. |
| `attribute_mismatch`  | A condition expression evaluated to false. |
| `role_missing`        | The required role was not assigned to the subject. |

## Lockout immunity

The admin REST surface is **role-gated, not permission-gated**.
A misconfigured rule can deny `admin` everything and the admin can
still `DELETE /v1/authz/rules/{id}` to fix it. This is enforced by the
`admin-cannot-lock-themselves-out` smoke test.

## Hard rules (from SCOPE.md)

| Rule | Summary |
|------|---------|
| R1   | Authz runs **after** auth — `Principal` is required. |
| R2   | Trait lives in `starter-spi`; impls live in `starter-authz`. |
| R3   | Unknown resources default-deny; **deny wins** on conflict. |
| R4   | Resources are **registered**, not stringly-discovered. |
| R5   | Two enforcement points: route middleware + in-handler. |
| R6   | TOML policy shape == DB table shape (round-trippable). |
| R7   | Built-in Reader/Writer/Admin available with zero config. |
| R8   | OAuth claims surface under `Principal.extra.oauth.*`. |
| R9   | Decisions emit stable reason codes. |
| R10  | Comments explain **why**, not what. |

## See also

- [`SCOPE.md`](./SCOPE.md) — full design, phasing plan, smoke tests.
- [`starter-authz` crate](../../../crates/starter-authz/) — implementation.
- [`starter-spi::authz`](../../../crates/starter-spi/src/authz/) — trait surface.
- [`starter-auth-oauth::OAuthPrincipalExtras`](../../../crates/starter-auth-oauth/src/principal_extras.rs) — OAuth attribute bridge (R8).
