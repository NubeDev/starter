# starter-authz — Scope Extension (Phase 7+)

## One-line summary

`SCOPE.md` documents Phases 1–6, which shipped: a policy engine with
RBAC + ownership, attribute conditions, a DB-backed policy store, and
an admin REST surface. This extension covers four additions that
SCOPE.md treats as **deferred or out-of-scope** and that real
deployment of the project's Niagara-style cloud product depends on:

1. **Tenants** — first-class `tenant_id` on `Principal`, `ResourceRef`,
   rules, and resource tables. Cross-tenant access is denied by
   construction.
2. **Teams** — group membership as a rule subject. `(team:hvac-ops,
   weather, refresh) → allow` is one row, not one per user.
3. **Decision audit log** — append-only `starter_authz_decisions`,
   queryable per tenant / subject / time, with sampled allows and
   retained denies.
4. **AuthZ-aware extension REST adapter** — `ContributeRest.auth`
   gains a `permission: { resource, action }` field; the rest router
   wraps each entry with `with_permission` automatically. Removes
   the host-side hand-mounting that `examples/authz-demo` does today.

Each is independently mergeable. Each replaces a workaround that
exists in the codebase or the demo today.

## Why this exists (now)

`SCOPE.md` "Open questions" lists four of these as deferred —
multi-workspace policies, durable audit, caching across processes,
and one not listed (teams). They were deferred for a single-operator
appliance shape. The project's current shape (per the project memory):

- **Multi-tenant cloud** for energy / water / HVAC dashboards.
- **End users (not just operators) author pages, flows, and rules.**
- **Third-party extensions** add their own resource kinds at runtime.
- **MCP, CLI, and gRPC** surfaces share the same authz decisions.

In that shape:

- Without **tenants**, a single misconfigured rule can leak data
  across customers. This is the most expensive bug a B2B SaaS can
  ship; retrofitting tenancy after data lands is painful (every
  resource table needs a backfill + uniqueness rework).
- Without **teams**, ops scale linearly with users — a 50-user
  customer needs 50 rules per resource. Operators won't tolerate it
  and will work around the system with shared accounts.
- Without **audit**, "why was alice denied X yesterday?" requires
  re-running the request. SOC 2 / ISO 27001 also expect a queryable
  authz audit trail.
- Without an **authz-aware extension adapter**, every extension
  developer either bypasses `starter_ext_server::rest_router` (the
  pattern in `examples/authz-demo/src/weather.rs`) or settles for
  `require_role`. The first scales badly; the second can't express
  per-user grants.

This document specifies the four additions and their interaction. It
does not respec the engine, the condition language, or the existing
admin REST routes — those stay as `SCOPE.md` defines them.

## Relationship to existing crates

```
starter-spi
   │  + Principal.tenant_id        (Option<String>)
   │  + Principal.teams            (Vec<String>)
   │  + ResourceRef.tenant         (Option<String>)
   │  + ResourceSpec.tenant_scoped (bool)            ← per kind
   │
   ├── starter-authz
   │     + condition vocabulary:   tenant, principal.teams contains "X"
   │     + StoredRule.tenant       (Option<String>)
   │     + DbPolicyEngine: tenant predicate before role evaluation
   │     + DecisionSink trait + DbDecisionSink writes to
   │       starter_authz_decisions
   │
   ├── starter-auth-users
   │     + starter_auth_users_tenants table
   │     + starter_auth_users_memberships (user_id, tenant_id, role)
   │     + starter_auth_users_teams (id, tenant_id, name)
   │     + starter_auth_users_team_members (team_id, user_id)
   │     + AuthAuthenticator populates Principal.tenant_id + .teams
   │     + admin REST: /v1/tenants, /v1/tenants/{id}/teams,
   │                   /v1/tenants/{id}/members
   │
   └── starter-extensions/starter-ext-server
         + ContributeRest.auth.permission: { resource, action }
         + rest_router wraps each entry with with_permission(...)
           when a permission is declared; layer order:
             with_role/with_scope (outer) → with_permission (inner)
```

Everything is **strictly additive** behind cargo features. A consumer
running Phase 1–6 as it stands today continues to work unchanged:

- `Principal.tenant_id` defaults to `None`; rules without a `tenant`
  field still evaluate.
- `Principal.teams` defaults to `[]`; conditions referencing
  `principal.teams` simply don't match.
- The decision sink is `NoopDecisionSink` by default; turning audit
  on is one wire-in.
- The `ContributeRest.auth.permission` field is optional; manifests
  without it behave exactly as today.

## Hard rules (load-bearing)

### R11 — Tenants are a first-class predicate, not an attribute

The naive design is "stuff `tenant_id` into `Principal.extra` and
write `extra.tenant == object.tenant` conditions everywhere." Don't.
Tenancy has three properties that the attribute bus cannot express
cheaply:

1. **Default-deny on missing-tenant** — a resource row without a
   `tenant_id` column is a deployment bug, not a "rule didn't
   match." Treating tenancy as a typed field on `ResourceRef`
   surfaces the bug at compile time / migration time, not as a
   silent allow.
2. **Predicate ordering** — the engine evaluates tenant **before**
   role / condition. A wrong-tenant request short-circuits to
   `Deny { reason: "cross_tenant" }` without consulting any rule.
   That guarantees a single misconfigured `role: "*", resource: "*"`
   allow cannot leak across tenants.
3. **Query pushdown** — list endpoints filter `WHERE tenant_id =
   $principal.tenant_id` before any rule fires. The engine cannot
   push attribute-bag predicates into SQL; it can push a typed
   `ResourceRef.tenant` predicate.

`ResourceSpec.tenant_scoped: bool` is declared per kind. For tenant-
scoped kinds, the engine **requires** `ResourceRef.tenant ==
Some(principal.tenant_id)`; missing or mismatching is `Deny {
reason: "cross_tenant" }`. For non-tenant-scoped kinds (e.g. the
admin singleton, system metrics) the field is ignored.

The reason for the boolean per kind: not every resource is tenant-
scoped. `users`, `tenants`, `extensions` are global. Forcing every
resource through tenancy would make the cross-tenant deny fire on
legitimate cross-tenant admin actions.

**`Principal.tenant_id == None` against a tenant-scoped resource is
`Deny { reason: "no_tenant_binding" }`.** Not "cross_tenant"
(there's no tenant to be wrong-side-of), not silent allow. This is
the case where the consumer has wired `starter-auth-token` (which
mints a tenantless `Principal`) but declared a resource as
`tenant_scoped = true`: the engine refuses every request to that
resource until the consumer fixes their wiring. The "strictly
additive" claim holds because a consumer not using tenancy never
sets `tenant_scoped = true` on any resource and so never hits this
deny path. A consumer who **opts into** tenant-scoped resources
opts into the requirement that every authenticator produce a
tenant-bound `Principal`.

The engine does **not** refuse to start when an authenticator can
produce `None` and a resource is tenant-scoped — the check is at
request time, not boot time, because the choice of authenticator
per route is a consumer wiring decision the engine can't statically
verify. A boot-time lint (`starter-cli authz check-wiring`) is a
nice-to-have, not a hard rule.

### R12 — Tenant ownership is two columns, not one

Every tenant-scoped resource table grows **two** columns:

- `tenant_id NOT NULL` — the tenant the row belongs to. Indexed,
  participates in unique constraints, gated by R11.
- `owner_id NOT NULL` — the subject who created the row. Drives
  ownership rules per SCOPE.md R4.

A row's `(tenant_id, owner_id)` is set at INSERT and never
mutated. **A constraint in prose is not a constraint** — the
migration that adds the two columns also installs a DB-level
guard:

- Postgres: `CREATE TRIGGER ... BEFORE UPDATE OF tenant_id,
  owner_id ... RAISE EXCEPTION 'tenant_id / owner_id are
  immutable'`.
- SQLite: equivalent `BEFORE UPDATE` trigger via `RAISE(ABORT,
  '...')`.

The engine matches `principal.subject == object.owner` for
ownership rules **only after** `principal.tenant_id ==
object.tenant` has cleared — so an owner moving tenants (which the
trigger now refuses) cannot accidentally reach their old rows.

Existing built-in tables (`reports`, `flows`, `pages`, the various
warehouse catalogs) backfill `tenant_id` from a chosen default
tenant in a one-shot migration. The migration is part of the
"enable Phase 7" rollout, not part of every consumer's normal
upgrade path.

**Informally multi-tenant deployments need a manual mapping
migration.** A deployment that has been splitting data through some
ad-hoc convention (e.g. a `workspace_id` column on `pages`, a
prefix on resource ids) will lose that separation if the migration
backfills one default tenant for every row. The default-tenant
backfill path is for **single-tenant → multi-tenant first-cutover
only**. Anyone else writes their own mapping script keyed off
their existing column; the SQL is theirs because the convention is
theirs.

### R13 — Teams are subjects in the rule grammar

A team is a named, tenant-scoped collection of users. Membership
is read-mostly (managed by tenant admins). The rule grammar gains
**one** new shape:

```toml
[[rules]]
role      = "*"
resource  = "weather"
actions   = ["refresh"]
condition = 'principal.teams contains "hvac-ops"'
effect    = "allow"
```

Teams are **not** roles. A user's `Principal.role` stays the
coarse `Reader | Writer | Admin`; team membership is a separate
list on `Principal.teams: Vec<String>`. Two reasons:

1. Roles are a workspace-wide invariant ("can this user access the
   admin slice"); teams are tenant-scoped, dynamic, and not
   evaluated by `require_role`.
2. Teams compose. A user can be in `hvac-ops` and `weather-
   readers` simultaneously; the existing role enum is exclusive.

`Principal.teams` is populated by the authenticator at session-
mint / token-verify time from `starter_auth_users_team_members`.
The condition mini-language gains a single new operator:

```text
expr := … | path 'contains' value
```

`contains` is only valid when the left-hand path resolves to a
JSON array. Anything else is a malformed-rule error at engine
compile time, not a silent false at evaluation. (Same shape as
`oauth.email_domain == "acme.com"` failing loudly when the attr
is missing — SCOPE.md R8.)

### R14 — Decisions go to a best-effort sink, denies unsampled

The engine emits a `Decision` for every check. Today that decision
goes to `tracing` only. Phase 7 adds a `DecisionSink` trait the
engine hands the entry to **after** the decision is computed but
**before** `check()` returns — the dispatch is `tokio::spawn` /
channel-send, not an awaited blocking write. The request path
never blocks on the sink. Pick this trade-off explicitly:

- **best-effort** — sink writes can be dropped on overflow without
  the engine returning an error or denying the request;
- **deny-asymmetric** — the shipped DB sink retains 100% of denies
  and a 1-in-N sample of allows, so the most security-relevant
  half of the log is durable up to the queue's overflow point;
- **fail-open on sink errors** — a sink that returns an error
  emits a `tracing::error` but does not affect the request's
  Allow/Deny outcome.

A consumer that needs **fail-closed durable audit** (regulated
workloads) wires a custom sink whose `record` blocks the request
path and returns an error, plus a wrapping engine that maps that
error to `Deny { reason: "audit_unavailable" }`. The shape is
supported; it is not the default because most consumers prefer
"no audit row" over "no service."

```rust
#[async_trait]
pub trait DecisionSink: Send + Sync {
    /// Record one decision. The default behaviour required by R14
    /// is non-blocking: implementations buffer + drop on overflow
    /// rather than slow the request path. A fail-closed wrapper is
    /// the consumer's choice, not the trait's default.
    async fn record(&self, entry: DecisionEntry);
}

pub struct DecisionEntry {
    pub at:             chrono::DateTime<chrono::Utc>,
    pub tenant:         Option<String>,
    pub subject:        String,
    pub principal_role: String,
    pub action:         String,
    pub kind:           String,
    pub id:             Option<String>,
    pub effect:         Effect,         // Allow | Deny
    /// Matched rule id when the decision came from a rule
    /// (audit-friendly dashboards key off this).
    pub rule_id:        Option<String>,
    /// Engine-supplied reason code when the decision came from
    /// engine semantics (`"cross_tenant"`, `"no_tenant_binding"`,
    /// `"unknown_resource"`, `"no_matching_rule"`). Independent
    /// from `rule_id` so a rule whose id happens to be
    /// `"cross_tenant"` is never confused with the built-in code.
    pub reason:         Option<String>,
}
```

Two shipped impls:

- `NoopDecisionSink` (default) — silently drops. Zero-overhead opt-
  in matches Phase 1–6's "you pay nothing if you don't enable it."
- `DbDecisionSink` — appends to `starter_authz_decisions`. Bounded
  channel (default 4096) + dedicated writer task; drop with
  `tracing::warn { dropped_count }` on overflow. **Never** blocks
  `check()`.

Retention policy lives in the same crate as the sink. The shipped
DB sink retains every `Deny` for **90 days** (default,
configurable) and a 1-in-N sample of `Allow` (default `N=100`).
The cleanup mechanism is a **scheduled task** wired in by
`starter_authz::audit::spawn_retention(pool, config)` and intended
to be called from the binary's startup alongside other background
tasks. The task runs hourly, deletes by `at < now() - retention`
in bounded batches (10k per run), and logs the count. **If the
binary never spawns this task, the table grows without bound** —
the doc names the dependency explicitly so the omission shows up
in code review, not at disk-full o'clock.

Per-tenant override: `STARTER_AUTHZ_DECISION_ALLOW_SAMPLE` is the
default; a tenant on regulated workloads can override it to `1`
(retain every allow) via `tenants.audit_allow_sample` column.
Cost is the tenant's to absorb.

**Auditor caveat (acknowledged limitation):** sampled allow logs
**cannot** answer "did alice ever successfully access X." They can
answer "was alice ever denied X" (denies are unsampled) and "what
was alice's allow rate against X" (sample is statistically
representative). A consumer whose auditor asks the first question
must turn the sample to 1 for the affected tenant — this is a
deliberate trade-off, not a bug.

### R15 — Extension REST entries can declare a permission gate inline

`ContributeRest.auth` today is `{ require_role, require_scope }`. It
gains one optional field:

```yaml
contributes:
  rest:
    - id: com.acme.weather.forecast
      method: GET
      path: /weather/forecast
      auth:
        permission:
          resource: weather
          action:   read
    - id: com.acme.weather.refresh
      method: POST
      path: /weather/refresh
      auth:
        permission:
          resource: weather
          action:   refresh
```

The REST adapter (`rest_router`) wraps each entry's sub-router with
`with_permission(resource, action)` automatically. Layer order
inside the adapter:

```
with_role (outer, from require_role)
  → with_scope (from require_scope)
    → with_permission (from permission)        ← NEW
      → handler
```

The `permission.resource` must be a `kind` the host registered in
its `ResourceRegistry`. The adapter calls `registry.lookup(kind)`
at build time; an unknown kind is a `RestBuildError::UnknownResource`
(same shape as `UnknownRole` today). This catches the typo at
deploy time, not at request time.

This rule replaces the `examples/authz-demo/src/weather.rs`
hand-mounting pattern. The demo's host-side router becomes a
docstring noting "this is what the adapter does for you now."

**Why not let extensions ship the `ResourceSpec` themselves?**
That's a Phase 8 question (per `SCOPE.md` Phase 4 — already
sketched there as `ExtensionContext::resources()`). R15 deliberately
covers only the consumption side: an extension declares which
`(resource, action)` its endpoint maps to; the **host** still
controls which kinds exist. This keeps the security boundary at
the host. A future phase can let extensions register kinds; the
manifest field shipped here is the same one.

## Data model additions

```sql
-- starter-auth-users migrations (sqlite shown; postgres analogous):

CREATE TABLE starter_auth_users_tenants (
  id           TEXT PRIMARY KEY,
  slug         TEXT NOT NULL UNIQUE,         -- url-safe identifier
  display_name TEXT NOT NULL,
  created_at   TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE starter_auth_users_memberships (
  tenant_id  TEXT NOT NULL REFERENCES starter_auth_users_tenants(id),
  user_id    TEXT NOT NULL REFERENCES starter_auth_users_users(id),
  role       TEXT NOT NULL,                  -- reader | writer | admin
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (tenant_id, user_id)
);
CREATE INDEX idx_memberships_user ON starter_auth_users_memberships (user_id);

CREATE TABLE starter_auth_users_teams (
  id           TEXT PRIMARY KEY,
  tenant_id    TEXT NOT NULL REFERENCES starter_auth_users_tenants(id),
  slug         TEXT NOT NULL,                -- url-safe; unique within tenant
  display_name TEXT NOT NULL,
  created_at   TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (tenant_id, slug)
);

CREATE TABLE starter_auth_users_team_members (
  team_id    TEXT NOT NULL REFERENCES starter_auth_users_teams(id),
  user_id    TEXT NOT NULL REFERENCES starter_auth_users_users(id),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (team_id, user_id)
);
CREATE INDEX idx_team_members_user ON starter_auth_users_team_members (user_id);

-- starter-authz migration:

ALTER TABLE starter_authz_rules
  ADD COLUMN tenant_id TEXT;   -- NULL = global rule; else scoped

CREATE INDEX idx_authz_rules_tenant ON starter_authz_rules (tenant_id);

CREATE TABLE starter_authz_decisions (
  id            TEXT PRIMARY KEY,
  at            TEXT NOT NULL,
  tenant_id     TEXT,
  subject       TEXT NOT NULL,
  principal_role TEXT NOT NULL,
  action        TEXT NOT NULL,
  kind          TEXT NOT NULL,
  resource_id   TEXT,
  effect        TEXT NOT NULL,                -- allow | deny
  reason        TEXT
);
CREATE INDEX idx_authz_decisions_tenant_at
  ON starter_authz_decisions (tenant_id, at);
CREATE INDEX idx_authz_decisions_subject_at
  ON starter_authz_decisions (subject, at);
CREATE INDEX idx_authz_decisions_effect_at
  ON starter_authz_decisions (effect, at);    -- "find recent denies"
```

`Principal.tenant_id` and `Principal.teams` are populated by the
authenticator from the current session's `(user_id, tenant_id)`
membership row. **A user with no membership for the requested
tenant is `Unauthenticated`, not `Forbidden`** — they can't even
prove identity to that tenant.

Multi-tenant session model: a session is `(user_id, tenant_id)`,
not `(user_id)`. The user picks a tenant at login (or there's only
one); the session cookie / token carries the tenant binding; the
authenticator surfaces it. Switching tenants is a re-login.

## Routes

New admin REST under the same `/v1/authz/*` and `/v1/auth/*`
prefixes:

```
POST   /v1/tenants                          create tenant (super-admin only)
GET    /v1/tenants                          list tenants visible to caller
GET    /v1/tenants/{id}
PATCH  /v1/tenants/{id}                     rename, slug, …
POST   /v1/tenants/{id}/members             add user as member with role
DELETE /v1/tenants/{id}/members/{user_id}
PATCH  /v1/tenants/{id}/members/{user_id}   change role
POST   /v1/tenants/{id}/teams               create team in tenant
DELETE /v1/tenants/{id}/teams/{team_id}
POST   /v1/tenants/{id}/teams/{team_id}/members      add user to team
DELETE /v1/tenants/{id}/teams/{team_id}/members/{user_id}

GET    /v1/authz/decisions?tenant=…&subject=…&effect=deny&since=…   page audit
```

`/v1/authz/decisions` is paginated by `at` (cursor) with bounded
`limit`. Tenant scoping: a tenant-admin sees decisions for their
own tenant; a super-admin sees everything. The route honours its
own gates — looking at the audit log is itself an audited action.

## Configuration

Three new env vars (all optional):

```
STARTER_AUTHZ_DECISION_SINK         "off" | "db" (default "off")
STARTER_AUTHZ_DECISION_RETAIN_DAYS  default 90
STARTER_AUTHZ_DECISION_ALLOW_SAMPLE default 100   (1 in N allows persisted)
```

A future Phase exposes these via `/v1/authz/config` for live tuning;
not in scope here.

## Extension story

An extension that contributes REST routes today writes:

```yaml
contributes:
  rest:
    - id: com.acme.weather.forecast
      method: GET
      path:   /weather/forecast
```

That worked in Phase 1–6 only by either accepting `require_role`-
granularity or having the host hand-mount the route (the demo's
approach). After R15:

```yaml
contributes:
  rest:
    - id: com.acme.weather.forecast
      method: GET
      path:   /weather/forecast
      auth:
        permission: { resource: weather, action: read }
```

…and that's it. The host registers the `weather` resource in its
`ResourceRegistry`; the adapter wraps the route in
`with_permission("weather", "read")`; per-user grant / revoke
through the policy engine works exactly like a built-in route.

**MCP and gRPC parity (per the project's MCP / gRPC goals):** the
same `permission: { resource, action }` declaration is consulted
by `starter-ext-mcp` and `starter-ext-grpc`. The engine `check()`
runs at the adapter boundary; the extension closure never sees an
unauthorised call. (gRPC adapter today does no authz; this is part
of the work.)

## Flow (a request through the stack, post-Phase-7)

```
HTTP request
  ↓
with_principal           (resolves session/token → Principal with tenant + teams)
  ↓
with_permission(R, A)    (calls engine.check → engine consults tenant predicate
  ↓                        first, then role + condition + ownership)
                         (engine writes a DecisionEntry via DecisionSink)
  ↓
handler                  (gets Principal with tenant/teams; uses
                          check_or_deny on row loads for ownership)
```

For an extension route, the `with_permission` layer is applied by
the rest adapter from the manifest — invisible to the extension
author.

## Smoke tests (before merging)

### "Cross-tenant request is denied before any rule evaluates"

Two tenants, two users, one shared rule `role:"*"` `resource:"*"`
`actions:["*"]` `effect:"allow"`. User from tenant A requests a row
owned by tenant B. Engine returns `Deny { reason: "cross_tenant" }`
without consulting the rule.

### "Team grant covers every team member"

`(team:hvac-ops, weather, refresh) → allow` is one rule. Add a new
user to the team; they immediately get `refresh`. Remove them;
they immediately lose it. No new rule rows.

### "Audit log records every deny"

Sink in "db" mode. Issue a request that denies. The
`starter_authz_decisions` row exists before the HTTP response is
returned to the client. (The test asserts ordering: SELECT
COUNT(*) after a 403, must be ≥ 1.)

### "Audit log samples allows"

Sink in "db" mode, sample = 10. Issue 1000 allow requests.
`SELECT COUNT(*) FROM starter_authz_decisions WHERE effect =
'allow'` is in `[80, 120]` — within a binomial spread.

### "Extension REST entry with `permission:` gets a permission gate"

Two extensions, each with one route and a `permission:`
declaration. A reader gets 200 on the `read` route, 403 on the
`refresh` route. Switch their role assignment to writer + grant
`refresh`; both pass. No host-side hand-mounting.

### "Extension declares unknown resource → deploy-time error"

Manifest with `permission: { resource: "doesnt_exist", action:
"read" }` makes `rest_router::build` return
`RestBuildError::UnknownResource { entry, kind }`. Server logs the
error, mounts the rest of the extensions, refuses to mount the
broken one. (Symmetric with today's `UnknownRole` behaviour.)

### "Switching tenants is a re-login"

A user belongs to two tenants. Their session/token binds to
exactly one. A request to a resource in the other tenant returns
401 (`Unauthenticated`), not 403 — the principal has no
authenticated identity for that tenant.

## Non-goals (for this extension)

- **Row-level filtering helper / query pushdown.** A future
  `engine.filter_query(p, action, kind) -> Predicate` would let
  list endpoints push tenant + owner filters into SQL automatically.
  This is the right shape, but it touches every CRUD endpoint and
  is best landed once the tenant + audit primitives are in.
- **User-managed share / delegation primitive.** "Alice shares her
  page with bob until next Friday" wants a `shares` table that's
  user-managed (not admin-managed like rules). Out of scope; lands
  as a separate ADR.
- **Multi-instance engine cache invalidation.** The
  `examples/authz-demo` caveat (CLI mutates DB; running server's
  cache doesn't reload) is fixed for the single-server case by
  going through the admin REST routes (Phase 3 already). The
  multi-server case wants Postgres LISTEN/NOTIFY or Redis pub/sub.
  Out of scope; the boundary is "what does it take to ship the
  next milestone."
- **Casbin / OPA bridges.** Phase 5 placeholder in `SCOPE.md`;
  unchanged here.
- **Enforced extension isolation (WASM / process).** Phase 7 keeps
  trust-equivalent-to-host for built-in extensions, matching
  `SCOPE.md` R10 in the `starter-extensions` SCOPE. Real isolation
  is a separate workstream.

## Decisions made (and the reasons)

- **`Principal.tenant_id: Option<String>`, not `String`.** A
  consumer not using tenancy keeps the value `None`; rules without
  a `tenant` field still evaluate. Forces tenant-scoped resources
  to declare themselves (`ResourceSpec.tenant_scoped = true`) and
  the engine enforces the predicate only when declared.
- **Teams live in `starter-auth-users`, not `starter-authz`.** Team
  membership is identity data, not policy data. `starter-authz`
  reads it through `Principal.teams`; it doesn't own the tables.
  Mirrors how OAuth identities live in `starter-auth-oauth` and
  authz reads them via `Principal.extra`.
- **`Principal.teams: Vec<String>` (slug list), not `Vec<TeamId>`.**
  Rules in TOML / YAML reference teams by stable slug. UUIDs would
  force rule edits whenever a team is recreated; slugs survive
  re-creation as long as the operator picks the same slug.
- **Audit sink is a trait, not just a table.** The DB writer is the
  shipped impl, but a consumer routing decisions to Loki /
  CloudWatch / Datadog wires their own sink without forking the
  engine. Same shape as the rest of `starter-spi`.
- **Allow-sampling default = 1 in 100.** A tenant doing 10 req/s
  produces ~8.6k allow events per day at 1-in-100; ~860k at full
  retention. The latter is unaffordable; the former is queryable.
  Configurable — but the default has to be cheap enough that
  turning audit on doesn't fill a disk by week 2.
- **`permission:` on `ContributeRest.auth`, not on every contribute
  shape.** Tools and gRPC methods (`ContributeTool`, future
  `ContributeGrpc`) take the same shape. The field is declared in
  `starter-ext-spi`'s `AuthGate` and the three adapters consume it
  uniformly.
- **Cross-tenant deny is `Deny { reason: "cross_tenant" }`, not
  404.** A 403 with a reason code is the convention everywhere
  else; faking a 404 to hide tenant existence is a leak-prevention
  hack that costs operator clarity. If hiding tenant existence
  matters for a specific deployment, the gateway in front can
  rewrite 403→404; the engine stays honest.

## Open questions

- **Tenant slug vs id in routes.** `/v1/tenants/{id}` uses the
  uuid; admin UIs prefer slug. Lean towards slug-with-id-fallback
  parsing, like Stripe's `/v1/customers/{cus_…}` accepting both.
- **Sampling — uniform random vs deterministic hash.** Deterministic
  hash (`xxhash(subject) % N == 0`) gives stable "this subject is
  always sampled" which is useful for debugging. Uniform random
  gives statistical correctness. Probably ship deterministic; revisit
  if it skews dashboards.
- **Whether `principal.teams contains "X"` is the only new
  operator.** A `principal.teams intersect ["X","Y"]` shorthand
  would shave one OR per rule. Wait for the first real rule that
  needs it.
- **Audit log table size.** SQLite vs Postgres behaves very
  differently at 100M+ rows; an export-and-truncate scheme will be
  needed before then. Out of scope for this extension; flagged here
  so it's not forgotten.

## Phasing

Each phase is independently mergeable; the order matters because
later phases depend on earlier ones.

### Phase 7a — Tenants

- `Principal.tenant_id: Option<String>`.
- `ResourceRef.tenant: Option<String>` + `ResourceSpec.tenant_scoped: bool`.
- `StoredRule.tenant_id: Option<String>` + migration.
- Engine evaluates tenant predicate before role / condition.
- `starter-auth-users`: tenants + memberships tables;
  authenticator populates `Principal.tenant_id`.
- Admin REST: `/v1/tenants` + `/v1/tenants/{id}/members`.
- Smoke tests: cross-tenant-deny, multi-tenant-session-binding,
  global-resource-bypass (`tenant_scoped = false` cases).

Outcome: a multi-tenant deployment is possible. No team grants yet
(per-user rules per tenant remain the only granularity).

### Phase 7b — Teams

- `Principal.teams: Vec<String>`.
- `starter-auth-users`: teams + team_members tables; authenticator
  populates `Principal.teams` at session-mint time.
- Condition mini-language gains `path 'contains' value`.
- Admin REST: `/v1/tenants/{id}/teams` + member CRUD.
- Smoke tests: team-grant-coverage, team-membership-remove-takes-
  effect, team-rules-tenant-scoped.

Outcome: ops scale sub-linearly with users. Per-team rules replace
N per-user rules.

### Phase 7c — Decision audit log

- `DecisionSink` trait + `NoopDecisionSink` + `DbDecisionSink`.
- `starter_authz_decisions` table + migration.
- Engine calls sink inside `check()`.
- Retention + sampling honoured by the DB sink.
- `GET /v1/authz/decisions` admin route.
- Smoke tests: deny-always-recorded, allow-sampled-at-rate,
  audit-route-is-itself-audited.

Outcome: every authz decision is queryable. SOC 2 / ISO 27001 audit
trail story works.

### Phase 7d — AuthZ-aware extension REST adapter

- `ContributeRest.auth.permission: { resource, action }` field on
  the manifest.
- `rest_router` wraps each entry with `with_permission` when
  declared; layer order documented.
- `RestBuildError::UnknownResource` for typos.
- `examples/authz-demo` simplified: drop the host-side `weather.rs`
  hand-mounting; the manifest declares the permission inline.
- Smoke tests: per-entry-permission-applied, unknown-resource-is-
  build-error, role+permission-compose-correctly.

Outcome: extension authors get per-user authz with zero host
changes. `examples/authz-demo` becomes the canonical
demonstration of the full Phase 7 stack.

## Bottom line

**Four additions, each strictly additive: tenants as a typed first-
class predicate (R11–R12), teams as a rule subject (R13), an
inside-the-engine decision sink (R14), and a `permission:` field on
extension manifests so the rest adapter does the gating (R15). Every
Phase 1–6 deployment keeps working with no changes; every Phase 7
deployment gets the multi-tenant, team-grant, audit-trail, extension-
authz-by-manifest shape the project's Niagara-style cloud product
needs.**
