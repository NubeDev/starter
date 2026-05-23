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

**Why role outer, permission inner — and the audit consequence.** A
user who fails the role gate gets a 403 from `with_role` before
the permission middleware ever runs; the audit log will record a
role-deny (via `with_role`'s tracing) but **no permission deny
entry**. That's intended: the role gate is a coarse precondition
("can this user touch this surface at all"), and dashboards of
"permission denies" by definition exclude pre-role rejections.
Acknowledge this in dashboards or add a `with_role` audit hook
later if it matters; do not flip the layer order to fix it,
because doing so would force the engine to evaluate rules for
requests the role gate would have killed anyway (wasted work +
larger attack surface for the engine).

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
  slug         TEXT NOT NULL UNIQUE,         -- url-safe identifier; reserved-name list enforced at create time
  display_name TEXT NOT NULL,
  audit_allow_sample INTEGER,                 -- per-tenant override of STARTER_AUTHZ_DECISION_ALLOW_SAMPLE; NULL = use env default
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
  rule_id       TEXT,                          -- which rule matched, when one did
  reason        TEXT                           -- engine-supplied code: cross_tenant | no_tenant_binding | unknown_resource | no_matching_rule
);
CREATE INDEX idx_authz_decisions_tenant_at
  ON starter_authz_decisions (tenant_id, at);
CREATE INDEX idx_authz_decisions_subject_at
  ON starter_authz_decisions (subject, at);
CREATE INDEX idx_authz_decisions_effect_at
  ON starter_authz_decisions (effect, at);    -- "find recent denies"
CREATE INDEX idx_authz_decisions_rule_at
  ON starter_authz_decisions (rule_id, at);   -- "which rule is firing most"
```

**Reserved slug list.** Tenant slugs eventually appear in URLs
(`/{slug}/dashboard`, future SDUI deep links). The `tenants`
INSERT path enforces a hard-coded reserved list — `admin`, `api`,
`auth`, `v1`, `v2`, `static`, `health`, `metrics`, `openapi`,
`extensions`, `mcp`, `tools`, `default`, `system` — plus anything
matching `[0-9]+` (collides with id-style routing). Adding to the
list later is a breaking change for any tenant who already grabbed
the slug; the only way to avoid that is to reserve broadly up
front. (Same trade-off GitHub made — `/api`, `/settings`, etc. are
all reserved usernames.)

`Principal.tenant_id` and `Principal.teams` are populated by the
authenticator from the current session's `(user_id, tenant_id)`
membership row. **A user with no membership for the requested
tenant is `Unauthenticated`, not `Forbidden`** — they can't even
prove identity to that tenant.

Multi-tenant session model: a session is `(user_id, tenant_id)`,
not `(user_id)`. The user picks a tenant at login (or there's only
one); the session cookie / token carries the tenant binding; the
authenticator surfaces it. Switching tenants is a re-login.

### Bearer-token bindings (cookie sessions are easy; tokens aren't)

Cookie sessions are minted fresh on each login, so the
`(user_id, tenant_id)` binding falls out for free. Bearer tokens
(`starter-auth-token` PATs, `starter-auth-users` API tokens,
external-IdP OAuth access tokens) need explicit handling. The
rules below are deliberate; they are not derivable from the
existing token verifiers and must land alongside the tenants
migration:

1. **PATs and API tokens are minted bound to one `(user_id,
   tenant_id)`.** The token-issue flow gains a `tenant_id`
   argument; the database column is `NOT NULL` for any token
   created post-migration. A super-admin token can be minted with a
   sentinel `tenant_id = "*"` (allowed only for users with the
   global `Admin` role); the engine treats `*` as "tenant predicate
   passes for any value." This is the one place the `Option<String>`
   shape is not enough.
2. **Membership-change revokes every active token for that
   `(user_id, tenant_id)`.** Adding or changing a membership is one
   write; **revoking** a membership is two writes — delete the
   membership row, then `UPDATE … SET revoked_at = now() WHERE
   user_id = ? AND tenant_id = ?` on the tokens table. Without
   this, a revoke takes effect only at token expiry, which is the
   month-3 security-incident shape. The token store grows a
   `revoke_for_membership(user_id, tenant_id)` method; the
   membership route calls it inside the same transaction.
3. **External-IdP OAuth tokens are not naturally tenant-bound.** A
   token from Google for `alice@acme.com` says nothing about which
   tenant alice is acting on behalf of. The OAuth callback path
   resolves the tenant via either (a) an explicit `?tenant=…` query
   param on the authorize URL, validated against alice's
   memberships, or (b) a "you have N tenants, pick one" interstitial
   on first-login. The resolved tenant is written into the local
   session row at callback time, **not** read from the IdP token on
   every request. This means the rest of the system never sees a
   tenantless OAuth principal; it sees the local session with the
   resolved binding.

The token table schema gets a `tenant_id TEXT NOT NULL` column and
the same `BEFORE UPDATE` immutability trigger as R12. The
membership-revoke path is a Phase 7a smoke test
(`token-revoked-when-membership-removed`).

## Routes

New admin REST under the same `/v1/authz/*` and `/v1/auth/*`
prefixes:

```
POST   /v1/tenants                          create tenant (super-admin only)
GET    /v1/tenants                          list tenants visible to caller
GET    /v1/tenants/{id}
PATCH  /v1/tenants/{id}                     rename, audit_allow_sample, …  (slug is immutable)
POST   /v1/tenants/{id}/members             add user as member with role
DELETE /v1/tenants/{id}/members/{user_id}   (also revokes that user's tokens for this tenant — see Bearer-token bindings)
PATCH  /v1/tenants/{id}/members/{user_id}   change role
POST   /v1/tenants/{id}/teams               create team in tenant
DELETE /v1/tenants/{id}/teams/{team_id}
POST   /v1/tenants/{id}/teams/{team_id}/members      add user to team
DELETE /v1/tenants/{id}/teams/{team_id}/members/{user_id}

GET    /v1/authz/decisions?tenant=…&subject=…&effect=deny&since=…   page audit
```

**`DELETE /v1/tenants/{id}` is deliberately not in this surface.**
Cascading deletion across every tenant-scoped table (reports,
flows, pages, marts, sandboxes, tokens, memberships, teams,
sessions, decisions, ...) is a high-blast-radius operation that
deserves an explicit ops workflow, not a one-button REST call.
The Phase 7a delivery is "tenants exist forever once created"; the
follow-up doc (`ADR-tenant-deletion`) covers the ordered-cascade,
the soft-delete-then-hard-delete window, and the operator
confirmation flow. Customers wanting to "delete their tenant" are
served by the soft-delete-then-disable path in the meantime
(`PATCH /v1/tenants/{id}` setting a `disabled_at` column blocks
all access; data lingers until the ADR lands).

**`GET /v1/authz/decisions` is exempt from allow-sampling.** A
tenant admin paging the deny log otherwise generates one
sampled-away allow per page request, which makes the audit-of-
audit chain ~99% lossy. The route's `with_permission` middleware
records its decision via a sink override that bypasses sampling
for this kind. (Implementation: a per-kind `sample_override` map
on the sink, defaulting to `audit_logs` → 1.)

`/v1/authz/decisions` is paginated by `at` (cursor) with bounded
`limit`. Tenant scoping: a tenant-admin sees decisions for their
own tenant; a super-admin sees everything. The route honours its
own gates — looking at the audit log is itself an audited action
(exempt from allow-sampling per the section above).

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

**MCP and gRPC parity is a goal, but a separate phase.** The
`permission: { resource, action }` field lives in
`starter-ext-spi`'s `AuthGate` so the MCP and gRPC adapters can
read it without further manifest churn. Wiring MCP and gRPC
adapters to call `engine.check()` at their dispatch boundary is
**Phase 7d.2** — listed separately because the gRPC adapter today
does no authz at all (not even `require_role`), so this is a
larger workstream than the REST add. Phase 7d ships the REST path
and the shared `AuthGate` field; Phase 7d.2 brings MCP and gRPC
to parity. A consumer needing per-user authz on MCP / gRPC routes
in the meantime mounts a host-side wrapper, just as
`examples/authz-demo` does for REST today.

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

### "Audit log eventually records every deny under no overflow"

Sink in "db" mode, default queue depth. Issue 100 deny requests
back-to-back. Poll `SELECT COUNT(*) FROM starter_authz_decisions
WHERE effect = 'deny'` with a 2s deadline; assert it reaches 100.
**The test is not ordering-sensitive against the HTTP response** —
the sink dispatch is non-blocking, so a row may land after the
response, but R14's best-effort contract says it lands as long as
the queue isn't overflowed. The 2s budget is generous enough to
catch the common-case write-lag without being flaky; a separate
soak test exercises the overflow path explicitly.

### "Audit log drops cleanly on overflow"

Sink in "db" mode, queue depth lowered to 4. Issue 1000 deny
requests in a tight loop with the writer task paused. Assert
`tracing::warn` emits `dropped_count` ≥ 1 and the server keeps
serving. (The contract is "drop, don't block" — the test proves
the drop, not the count.)

### "Audit log samples allows"

Sink in "db" mode, sample = 10. Issue 1000 allow requests.
`SELECT COUNT(*) FROM starter_authz_decisions WHERE effect =
'allow'` is in `[80, 120]` — within a binomial spread.

### "None-tenant principal hits tenant-scoped resource → no_tenant_binding"

A `starter-auth-token` authenticator (which produces tenantless
`Principal`s) wired in front of a router with a `tenant_scoped =
true` resource. Any request to that resource returns `Deny {
reason: "no_tenant_binding" }` — no rule consulted.

### "Membership-revoke kills the user's tokens"

Mint a PAT for `(alice, tenant-A)`. Delete the membership row.
Subsequent requests with that PAT return 401, **not** 403 — the
token is revoked, not merely unauthorized. Same check for
`starter-auth-users` API tokens. (OAuth tokens are covered by the
"session row owns the tenant binding" rule; not in this test.)

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
- **Team slugs are immutable after create.** A rename would
  silently break every rule referencing the old slug — the engine
  has no way to know "team X was renamed to Y." The team's
  `display_name` is mutable; the slug is the rule-stable identity.
  Tenant slugs are immutable for the same reason, plus the URL-
  routing concern.
- **Audit dispatch is non-blocking (best-effort, deny-asymmetric),
  not synchronous (fail-closed).** Synchronous audit forces every
  authz check to await a DB write; a sink outage takes the service
  down. Best-effort + 100% deny retention + sampled allows answers
  the deny-side compliance question without service coupling.
  Consumers with regulated workloads override via a fail-closed
  sink wrapper (sketched in R14).
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

**Note on 7b vs 7c order.** Audit (7c) has the only externally-
imposed timeline pressure — a customer / auditor / incident response
ask. Teams (7b) is a quality-of-life improvement; per-user rules
keep working until it lands. A consumer with an audit pressure
should re-order to `7a → 7c → 7b → 7d`; the doc keeps the
implementation-friendly order (7b first because the team-tables
migration is small and the condition-grammar change is a tight
diff), but the dependency graph permits either.

### Phase 7a — Tenants

- `Principal.tenant_id: Option<String>`.
- `ResourceRef.tenant: Option<String>` + `ResourceSpec.tenant_scoped: bool`.
- `StoredRule.tenant_id: Option<String>` + migration.
- Engine evaluates tenant predicate before role / condition.
- `(tenant_id, owner_id)` immutability triggers on every tenant-
  scoped table.
- `starter-auth-users`: tenants + memberships tables; tokens table
  grows `tenant_id NOT NULL`; membership-revoke also revokes tokens.
- Authenticator populates `Principal.tenant_id`.
- Admin REST: `/v1/tenants` + `/v1/tenants/{id}/members`. Slug
  reservation list enforced at create.
- Smoke tests: cross-tenant-deny, none-tenant-no_tenant_binding,
  multi-tenant-session-binding, global-resource-bypass
  (`tenant_scoped = false` cases), token-revoked-when-membership-
  removed, immutability-trigger-rejects-update.

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
- `DecisionEntry` with split `rule_id` + `reason` fields.
- `starter_authz_decisions` table + migration; per-tenant
  `audit_allow_sample` override column.
- Engine hands the entry to the sink before returning, but the
  dispatch is non-blocking (best-effort per R14).
- Retention task wired in by `audit::spawn_retention(pool, cfg)`.
- `GET /v1/authz/decisions` admin route, exempt from allow-sampling.
- Smoke tests: deny-eventually-recorded, deny-drops-cleanly-on-
  overflow, allow-sampled-at-rate, audit-route-not-sampled,
  retention-task-deletes-expired.

Outcome: every authz **deny** is queryable (under no overflow);
allow access patterns are statistically observable. Meets the
deny-side compliance ask without coupling the request path to the
audit DB. Customers who need 100% allow retention flip the per-
tenant sample to 1.

### Phase 7d — AuthZ-aware extension REST adapter

- `ContributeRest.auth.permission: { resource, action }` field on
  the manifest, declared in `starter-ext-spi`'s `AuthGate`.
- `rest_router` wraps each entry with `with_permission` when
  declared; layer order (role → scope → permission → handler)
  documented.
- `RestBuildError::UnknownResource` for typos.
- `examples/authz-demo` simplified: drop the host-side `weather.rs`
  hand-mounting; the manifest declares the permission inline.
- Smoke tests: per-entry-permission-applied, unknown-resource-is-
  build-error, role+permission-compose-correctly.

Outcome: REST extension authors get per-user authz with zero host
changes. `examples/authz-demo` becomes the canonical demonstration
of the full Phase 7 stack.

### Phase 7d.2 — MCP and gRPC adapter parity

- `starter-ext-mcp` calls `engine.check()` at tool-dispatch using
  the same `AuthGate.permission` field.
- `starter-ext-grpc` gets its first authz layer; the gRPC dispatcher
  invokes `engine.check()` keyed off the per-method `AuthGate`.
- Smoke tests (one per surface): mcp-permission-applied,
  grpc-permission-applied, surface-decisions-share-audit-trail.

Outcome: MCP and gRPC extension routes are authz-gated by the same
declaration REST already honours. Separated from 7d because gRPC
has no authz at all today, which is a meaningful workstream of its
own.

## Bottom line

**Four additions, each strictly additive: tenants as a typed first-
class predicate that defaults-deny on missing binding (R11–R12),
teams as a rule subject with immutable slugs (R13), a best-effort
decision sink with 100% deny retention + sampled allows (R14), and
a `permission:` field on extension manifests so the rest adapter
does the gating (R15). Every Phase 1–6 deployment keeps working
with no changes; every Phase 7 deployment gets the multi-tenant,
team-grant, deny-side-auditable, extension-authz-by-manifest shape
the project's Niagara-style cloud product needs. MCP and gRPC
parity lands in 7d.2 — the manifest shape is shared, the wiring
follows.**
