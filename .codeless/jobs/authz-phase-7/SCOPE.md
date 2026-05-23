# Scope — authz-phase-7

The authoritative design lives at
[/home/user/code/rust/starter/DOCS/auth/authz/SCOPE-EXT.md](/home/user/code/rust/starter/DOCS/auth/authz/SCOPE-EXT.md).
This brief is the trimmed per-job scope. Where this disagrees with
the source SCOPE, **the source SCOPE wins** — fix this file rather
than diverge.

## Goal

Land Phase 7 of starter-authz on the `starter` repo via the
`codeless/authz-phase-7` branch — the four strictly-additive
extensions called out in SCOPE-EXT.md that the Niagara-style cloud
shape depends on: **tenants** (7a), **teams** (7b), **decision
audit** (7c), and the **authz-aware extension REST adapter** (7d).
MCP and gRPC parity (7d.2) is bundled because its manifest field
ships in 7d and the wiring is the same `engine.check()` call at
the dispatch boundary.

After this job:

1. `Principal.tenant_id: Option<String>` + `Principal.teams:
   Vec<String>` + `ResourceRef.tenant: Option<String>` +
   `ResourceSpec.tenant_scoped: bool` exist on `starter-spi`.
2. The engine evaluates **tenant before role/condition**;
   cross-tenant requests short-circuit to `Deny { reason:
   "cross_tenant" }` without consulting any rule.
3. `Principal.tenant_id == None` against a `tenant_scoped = true`
   resource is `Deny { reason: "no_tenant_binding" }`.
4. `starter-auth-users` owns the tenants / memberships / teams /
   team_members tables. The authenticator populates
   `Principal.tenant_id` and `Principal.teams`. Tokens carry
   `tenant_id NOT NULL` and a `(user_id, tenant_id)`
   immutability trigger.
5. Membership-revoke also revokes that `(user_id, tenant_id)`'s
   tokens in the same transaction.
6. The rule condition grammar gains exactly one new operator:
   `path 'contains' value`, valid only when the left-hand path
   resolves to a JSON array (engine compile-time error
   otherwise).
7. `DecisionSink` trait + `NoopDecisionSink` (default) +
   `DbDecisionSink` (best-effort, 100% deny retention, 1-in-N
   allow sampling). `audit::spawn_retention(pool, cfg)` available
   for callers to schedule from their binary.
8. `GET /v1/authz/decisions` paginated audit query, scoped per
   caller's tenant (super-admin sees everything), exempt from
   allow-sampling.
9. `ContributeRest.auth.permission: { resource, action }` field
   on extension manifests; `rest_router` wraps each entry with
   `with_permission` automatically with the documented layer
   order (`with_role` outer → `with_scope` → `with_permission`
   inner → handler). `examples/authz-demo`'s host-side
   `weather.rs` hand-mounting is gone.
10. `RestBuildError::UnknownResource` for typo'd `resource` in a
    manifest.
11. MCP and gRPC adapters call `engine.check()` at their
    dispatch boundary keyed off the same `AuthGate.permission`
    field (Phase 7d.2 in the source SCOPE).
12. R11 through R15 in the source SCOPE hold by construction.

## In scope (five slices mirroring the source SCOPE phasing)

The source SCOPE is explicitly phased 7a → 7b → 7c → 7d (→ 7d.2);
the stages in `template.yaml` follow that phasing. Each phase is
independently mergeable in the source SCOPE's framing; in this
**one job** the phases are stages with no intermediate merge.

- **Slice 7a (stage 1) — Tenants:**
  - `Principal.tenant_id: Option<String>` on `starter-spi`.
  - `ResourceRef.tenant: Option<String>` +
    `ResourceSpec.tenant_scoped: bool` on `starter-spi`.
  - `StoredRule.tenant_id: Option<String>` + migration on
    `starter-authz`.
  - Engine evaluates the tenant predicate **before** role and
    condition (per R11). Two new typed deny reasons:
    `cross_tenant`, `no_tenant_binding`.
  - `starter-auth-users`: `starter_auth_users_tenants` +
    `starter_auth_users_memberships` tables. Slug reservation
    list enforced at INSERT (`admin`, `api`, `auth`, `v1`, `v2`,
    `static`, `health`, `metrics`, `openapi`, `extensions`,
    `mcp`, `tools`, `default`, `system`, plus `[0-9]+`).
  - Tokens table grows `tenant_id NOT NULL`. Token-issue flow
    takes a `tenant_id`. Super-admin sentinel `tenant_id = "*"`
    (allowed only for users with global `Admin`).
  - `(user_id, tenant_id)` immutability triggers on every
    tenant-scoped table (R12) — Postgres `BEFORE UPDATE` raise +
    SQLite equivalent.
  - Membership-revoke calls
    `token_store.revoke_for_membership(user_id, tenant_id)` in
    the same transaction (R-7a Bearer-token bindings).
  - OAuth callback resolves the tenant at callback time, writes
    it into the local session row (R-7a Bearer-token bindings
    item 3). Either `?tenant=…` validated against memberships
    or a "pick one" interstitial on first-login.
  - `AuthAuthenticator` populates `Principal.tenant_id`.
  - Admin REST: `POST/GET /v1/tenants`, `GET/PATCH
    /v1/tenants/{id}` (slug immutable; `audit_allow_sample`
    patchable), `POST /v1/tenants/{id}/members`, `DELETE
    /v1/tenants/{id}/members/{user_id}` (cascades
    token-revoke), `PATCH /v1/tenants/{id}/members/{user_id}`.
  - **No `DELETE /v1/tenants/{id}`** — explicitly deferred to
    `ADR-tenant-deletion`. Soft-disable via `PATCH` setting a
    `disabled_at` column is the meantime path.
  - Built-in tenant-scoped tables (`reports`, `flows`, `pages`,
    marts, sandboxes, sessions, decisions) grow `tenant_id NOT
    NULL` + `owner_id NOT NULL` per R12. One-shot migration
    backfills the default tenant. The migration docstring names
    the informally-multi-tenant manual-mapping caveat (R12).

- **Slice 7b (stage 2) — Teams:**
  - `Principal.teams: Vec<String>` (slug list, not ids) on
    `starter-spi`.
  - `starter-auth-users`: `starter_auth_users_teams` +
    `starter_auth_users_team_members` tables. Team slugs
    immutable after create; `display_name` mutable.
  - Authenticator populates `Principal.teams` at session-mint /
    token-verify time.
  - Condition mini-language gains `expr := … | path 'contains'
    value`. Engine-compile-time error if left-hand path doesn't
    resolve to a JSON array (R13, parallel to R8 in the parent
    SCOPE).
  - Admin REST: `POST /v1/tenants/{id}/teams`, `DELETE
    /v1/tenants/{id}/teams/{team_id}`, `POST /v1/tenants/{id}/
    teams/{team_id}/members`, `DELETE
    /v1/tenants/{id}/teams/{team_id}/members/{user_id}`.

- **Slice 7c (stage 4) — Decision audit log:**
  - `DecisionSink` trait on `starter-authz` with the exact
    shape in R14.
  - `DecisionEntry` with split `rule_id` (matched rule) and
    `reason` (engine-supplied code) fields.
  - `NoopDecisionSink` (default; silently drops, zero overhead).
  - `DbDecisionSink` — bounded mpsc (default depth 4096) +
    dedicated writer task. Drop with `tracing::warn {
    dropped_count }` on overflow. **Never blocks `check()`.**
  - `starter_authz_decisions` migration with the indices in the
    source SCOPE (`tenant_id, at`; `subject, at`; `effect, at`;
    `rule_id, at`).
  - `tenants.audit_allow_sample INTEGER` column on
    `starter_auth_users_tenants` (per-tenant override of the
    env default).
  - Sample policy: 100% denies, 1-in-N allows (default N=100
    via `STARTER_AUTHZ_DECISION_ALLOW_SAMPLE`). **Deterministic
    hash sampling** (`xxhash(subject) % N == 0`) per the
    "decisions made" open question — revisit if dashboards
    skew.
  - Per-kind `sample_override` map on the sink: `audit_logs`
    defaults to 1 (audit-of-audit must not lose entries).
  - `audit::spawn_retention(pool, cfg)` — hourly task, deletes
    `at < now() - retention` in bounded batches (10k per run),
    logs counts. Default retention 90 days
    (`STARTER_AUTHZ_DECISION_RETAIN_DAYS`).
  - `GET /v1/authz/decisions?tenant=…&subject=…&effect=deny&since=…`
    cursor-paginated by `at`, bounded `limit`. Tenant-admin
    sees own tenant; super-admin sees everything. Exempt from
    allow-sampling.
  - Three env vars: `STARTER_AUTHZ_DECISION_SINK`,
    `STARTER_AUTHZ_DECISION_RETAIN_DAYS`,
    `STARTER_AUTHZ_DECISION_ALLOW_SAMPLE`.

- **Slice 7d (stage 5) — REST adapter:**
  - `AuthGate.permission: Option<{ resource: String, action:
    String }>` on `starter-ext-spi`'s `AuthGate` (shared field
    used by REST, MCP, and gRPC adapters).
  - `ContributeRest.auth.permission` plumbed through manifest
    deserialization with `deny_unknown_fields`.
  - `rest_router::build`: when `permission` is set, wrap the
    sub-router in `with_permission(resource, action)`. Layer
    order documented in code: `with_role` (outer) → `with_scope`
    → `with_permission` (inner) → handler. The role-outer
    audit consequence (role denies don't appear as permission
    denies) is doc-commented on the layer wiring.
  - `permission.resource` validated against
    `ResourceRegistry::lookup(kind)` at build time;
    `RestBuildError::UnknownResource` on miss (symmetric with
    today's `UnknownRole`). Broken extension refuses to mount;
    rest of the host comes up.
  - `examples/authz-demo/src/weather.rs` simplified: drop the
    host-side `with_permission` mounting. Manifest declares
    the permission inline. Old behaviour becomes a `///`
    docstring on the now-empty wiring point ("this is what the
    adapter does for you now").

- **Slice 7d.2 (stage 6) — MCP + gRPC parity:**
  - `starter-ext-mcp`: call `engine.check()` at tool-dispatch
    using `AuthGate.permission`. Same layer-order
    documentation.
  - `starter-ext-grpc`: gain its first authz layer; gRPC
    dispatcher invokes `engine.check()` keyed off per-method
    `AuthGate`. gRPC's current shape has zero authz, so this is
    a larger workstream than the REST add — but the field is
    shared, the trait surface is shared, the wiring is the only
    new bit.

## Out of scope

- **Row-level filtering helper / query pushdown.** Future
  `engine.filter_query(p, action, kind) -> Predicate` is the
  right shape but touches every CRUD endpoint. Lands once Phase
  7 primitives are in.
- **User-managed share / delegation primitive** ("alice shares
  her page with bob"). Wants a `shares` table that's
  user-managed, not admin-managed. Separate ADR.
- **Multi-instance engine cache invalidation.** Single-server
  case is fixed by Phase 3 admin REST going through the engine.
  Multi-server case (Postgres LISTEN/NOTIFY or Redis pub/sub) is
  out of scope.
- **Casbin / OPA bridges.** Phase 5 placeholder in `SCOPE.md`;
  unchanged here.
- **Enforced extension isolation (WASM / process).** Out of
  scope per R10 in the extensions SCOPE.
- **`DELETE /v1/tenants/{id}`.** Deliberately not in the surface
  per the source SCOPE; covered by `ADR-tenant-deletion` as a
  separate workstream.
- **Live tuning via `/v1/authz/config`.** Future Phase per the
  source SCOPE; not in scope here.
- **`principal.teams intersect ["X","Y"]` shorthand.** Open
  question in the source SCOPE; wait for the first real rule
  that needs it.
- **Audit log table-size export-and-truncate strategy.** Source
  SCOPE flags this as a known scaling concern at 100M+ rows.
  Out of scope; retention task covers the steady-state case.

## Constraints

- **R11** — Tenants are a first-class predicate, not an
  attribute. `ResourceSpec.tenant_scoped: bool` per kind.
  Engine evaluates tenant before role/condition. `None` against
  tenant-scoped is `no_tenant_binding`; mismatch is
  `cross_tenant`.
- **R12** — Every tenant-scoped resource table grows **two**
  columns: `tenant_id NOT NULL` and `owner_id NOT NULL`. Both
  immutable after INSERT via a DB-level trigger (Postgres
  `BEFORE UPDATE` raise + SQLite `RAISE(ABORT, ...)`). A
  constraint in prose is not a constraint.
- **R13** — Teams are subjects in the rule grammar, not roles.
  `Principal.teams: Vec<String>` (slugs). One new operator:
  `path 'contains' value`. Engine compile-time error if the
  left-hand path doesn't resolve to a JSON array.
- **R14** — Decisions go to a best-effort sink, denies
  unsampled. Dispatch is non-blocking (`tokio::spawn` /
  channel-send). Default sink is `Noop`; shipped DB sink is
  best-effort with 100% deny retention and 1-in-N allow
  sampling. Fail-open on sink errors (`tracing::error` but
  no effect on Allow/Deny). Consumers needing fail-closed wire
  a custom sink.
- **R15** — Extension REST entries declare permission inline
  via `auth.permission: { resource, action }`. Layer order
  inside the adapter is `with_role` outer → `with_scope` →
  `with_permission` inner. Unknown resource is a deploy-time
  `RestBuildError::UnknownResource`. The role-outer audit
  consequence is documented, not fixed by reordering.
- **Strictly additive.** A Phase 1–6 consumer running today
  continues to work unchanged. `tenant_id` defaults `None`,
  `teams` defaults `[]`, sink defaults `Noop`, `permission`
  field defaults absent. Each phase is independently mergeable
  in the source SCOPE's framing; in this job the phases stay
  on one branch but each is a separate commit so they can be
  cherry-picked if the bundle is rejected.
- **R-trio applies** (CLAUDE.md): every stage ends with
  `checks`, `docs`, `git` per the closing trio block in
  `WORKFLOW.md`.
- **No `--no-verify` or `--force`.** If a pre-commit hook
  fails, fix the cause.
- **MSRV / lint gates**: `cargo test --workspace`,
  `cargo clippy --workspace --all-features -- -D warnings`,
  `cargo fmt --check` green at every stage boundary.
- **Migrations apply to BOTH SQLite and Postgres.** The source
  SCOPE shows SQLite syntax for brevity; the Postgres analogue
  ships in the same stage. Both immutability triggers (Postgres
  `BEFORE UPDATE … RAISE EXCEPTION` + SQLite `RAISE(ABORT,
  …)`) land together.
- **Token revoke is transactional with membership delete.** The
  source SCOPE is explicit: revoking a membership without
  revoking tokens is the "month-3 security-incident shape." A
  test (`token-revoked-when-membership-removed`) gates merge.

## Deliverables (what "done" looks like)

1. `codeless/authz-phase-7` branch with one commit per stage
   (six stages + one REVIEW handover = seven commits), pushed
   via mani.
2. `cargo test --workspace` green at every stage boundary
   (SQLite + Postgres testcontainers paths both green).
3. `cargo clippy --workspace --all-features -- -D warnings`
   green at every stage boundary.
4. `cargo fmt --check` green at every stage boundary.
5. **Slice 7a acceptance** — every smoke test in the source
   SCOPE under Phase 7a is implemented and green:
   `cross-tenant-deny`, `none-tenant-no_tenant_binding`,
   `multi-tenant-session-binding`, `global-resource-bypass`
   (`tenant_scoped = false` cases),
   `token-revoked-when-membership-removed`,
   `immutability-trigger-rejects-update`. Plus the OAuth
   callback path's tenant-resolution test (one of `?tenant=`
   validated, one of "pick-one" interstitial).
6. **Slice 7b acceptance** — `team-grant-coverage`,
   `team-membership-remove-takes-effect`,
   `team-rules-tenant-scoped` green. Engine compile-time error
   tested for `condition = 'principal.teams contains 42'` (LHS
   not a JSON array).
7. **Slice 7c acceptance** — `deny-eventually-recorded` (100
   denies, 2s deadline; non-ordering-sensitive contract per
   source SCOPE), `deny-drops-cleanly-on-overflow` (1000 denies
   with paused writer, asserts `tracing::warn` fires and server
   keeps serving), `allow-sampled-at-rate` (sample=10, 1000
   allows, count in `[80, 120]`), `audit-route-not-sampled`,
   `retention-task-deletes-expired`.
8. **Slice 7d acceptance** —
   `per-entry-permission-applied`, `unknown-resource-is-build-
   error`, `role+permission-compose-correctly`,
   `extension-rest-with-permission-no-host-handmount` (the demo
   weather route serves without `weather.rs` mounting).
9. **Slice 7d.2 acceptance** — `mcp-permission-applied`,
   `grpc-permission-applied`,
   `surface-decisions-share-audit-trail` (a deny via MCP and a
   deny via gRPC both land in `starter_authz_decisions` with
   the right `surface` distinguishable).
10. Module docstrings cite the rule numbers (`R11–R15`) from
    the source SCOPE.
11. `examples/authz-demo` updated: host-side `weather.rs`
    hand-mounting removed; the demo's host becomes the
    canonical Phase 7 wiring example (tenants + teams + sink +
    extension-permission, end to end).

## Open questions — RESOLVED (2026-05-23, before start)

The source SCOPE is unusually well-resolved — every "Decisions
made" item is locked, R11–R15 are explicit, and the phasing is
already laid out. Four job-specific resolutions follow.

### Q1 — One job or four?

**Answer: One job, six stages, one REVIEW gate. The source
SCOPE phasing (7a–7d.2) is the stage structure; the REVIEW
gate sits after 7a because tenant-scoped table migration is
the most expensive bug to ship (cross-tenant data leak is
unrecoverable without restore-from-backup).**

The source SCOPE says each phase is independently mergeable.
In this job they ride one branch so the integration story is
proven end-to-end before merge — but each stage is its own
commit so the bundle can be split if the second REVIEW reads
the diff and prefers a phase-by-phase merge cadence. The
REVIEW gate after 7a is non-negotiable: every later phase
depends on the tenant predicate being correct, and the
backfill migration in 7a is one-way.

**Decision.**
1. One job, six stages, one REVIEW gate.
2. Cap at **30000¢ / 8h** — double the standard cap because
   the migration footprint is large (every tenant-scoped table
   in the workspace; trigger ports for both SQLite and
   Postgres). Stage 1 (slice 7a) is ~40% of cap; stages 2–6
   share the remainder.
3. REVIEW gate after stage 1 (slice 7a). Gate question: does
   the tenant predicate fire correctly across every smoke
   test, does the backfill migration land cleanly on a real
   Postgres + a real SQLite, and is the cross-tenant deny
   provably untouchable by a wildcard rule?
4. If the budget is blown before slice 7d.2, halt at the
   second commit of slice 7d and split 7d.2 off as
   `authz-phase-7d2`. Do not silently land a partial 7d.2
   where MCP is wired but gRPC isn't (the
   `surface-decisions-share-audit-trail` test is the
   load-bearing acceptance for the bundle).

### Q2 — OAuth callback tenant resolution: query param or interstitial?

**Answer: ship both. The OAuth callback inspects
`?tenant=<slug>` first (validated against the user's
memberships); if absent and the user has exactly one
membership, auto-select; if absent and the user has multiple,
render the "you have N tenants, pick one" interstitial.**

The source SCOPE allows either; this job picks "both" because
the `?tenant=…` path is what API integrators want (they can
construct the authorize URL directly) and the interstitial is
what end-users need (they don't construct URLs). A user with
exactly one tenant should never see the interstitial — that's
friction.

The interstitial is a minimal HTML form in `starter-auth-users`
returning a `POST /v1/auth/oauth/select-tenant` that writes the
chosen tenant into the local session row. No SDUI / page
builder dependency; the form is hand-rolled HTML, not a
component.

### Q3 — Sampling: deterministic hash or uniform random?

**Answer: deterministic hash via `xxhash(subject) % N == 0`.**

The source SCOPE leans deterministic; this resolution commits
to it. Reasons:

1. Debugging — "alice's allows always sample" is a property a
   support engineer can reason about.
2. Reproducibility — a customer reporting "I made 1000 requests
   and only 8 appeared in audit" gets a deterministic answer
   instead of "binomial spread says that's within range."
3. Statistical correctness — deterministic per-subject sampling
   is **less** representative across the population than
   uniform random, but the source SCOPE explicitly says "revisit
   if it skews dashboards." This job ships deterministic; the
   follow-up is a flag, not a rewrite.

The hash function is `xxhash` (already in the workspace via
`xxhash-rust`). The seed is per-process (boot-time random) so
two different deployments don't sample identical subjects.
Per-tenant override (`audit_allow_sample = 1`) still works —
the hash is short-circuited when N=1.

### Q4 — Migration strategy for tenant-scoped tables already in production

**Answer: a single new migration per crate, gated on a
`STARTER_AUTHZ_PHASE_7_DEFAULT_TENANT` env var. If the var is
set, the migration backfills every row with that tenant slug
(must exist in `starter_auth_users_tenants` after the tenants
migration runs). If the var is unset, the migration refuses to
run and prints the manual-mapping caveat from R12.**

The source SCOPE is explicit that the default-backfill path is
"single-tenant → multi-tenant first-cutover only." Anyone with
ad-hoc multi-tenancy (e.g. a `workspace_id` column on `pages`)
writes their own mapping script. This job ships the
single-tenant cutover path; the manual-mapping path is a
docstring on the migration.

The env var is process-local. A consumer with no Phase 1–6
data installed just doesn't set it; the migration runs and the
backfill is empty (no rows to update). A consumer with Phase
1–6 data sets the var to their chosen default tenant's slug;
the migration creates that tenant (idempotent INSERT) then
backfills.

This is the only step that **requires** operator action between
"upgrade starter" and "boot succeeds" for an existing Phase 1–6
deployment. Documented in the migration's `--help` text and the
crate's README.

## References

- Source SCOPE (authoritative):
  [/home/user/code/rust/starter/DOCS/auth/authz/SCOPE-EXT.md](/home/user/code/rust/starter/DOCS/auth/authz/SCOPE-EXT.md)
- Parent SCOPE:
  [/home/user/code/rust/starter/DOCS/auth/authz/SCOPE.md](/home/user/code/rust/starter/DOCS/auth/authz/SCOPE.md)
- Crate layout (ground truth):
  - `/home/user/code/rust/starter/crates/starter-spi/`
  - `/home/user/code/rust/starter/crates/starter-authz/`
  - `/home/user/code/rust/starter/crates/starter-auth-users/`
  - `/home/user/code/rust/starter/crates/starter-auth-token/`
  - `/home/user/code/rust/starter/crates/starter-auth-oauth/`
  - `/home/user/code/rust/starter/starter-extensions/crates/starter-ext-spi/`
  - `/home/user/code/rust/starter/starter-extensions/crates/starter-ext-server/`
  - `/home/user/code/rust/starter/starter-extensions/crates/starter-ext-mcp/`
  - `/home/user/code/rust/starter/starter-extensions/crates/starter-ext-grpc/`
- Existing demo to simplify:
  `/home/user/code/rust/starter/examples/authz-demo/`
