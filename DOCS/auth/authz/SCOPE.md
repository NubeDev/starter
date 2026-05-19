# starter-authz — Scope

## One-line summary

`starter-authz` is a small Rust crate that adds a **policy-based
authorization layer** (subject × action × object → allow/deny) on
top of the existing `Authenticator` → `Principal` pipeline. It turns
the current binary "is the request authenticated, and does the
`Principal.role` clear a coarse bar?" model into a per-route,
per-resource, per-user **permission grid** that an operator can tick
boxes against — including for routes added later by extensions or
consumer application code.

The crate is **authorization, not authentication**. It runs *after*
whichever `Authenticator` the binary wires in (`starter-auth-token`,
`starter-auth-users`, `starter-auth-oauth`, future SAML / OIDC) has
produced a `Principal`, and decides whether that `Principal` is
allowed to perform `action` on `object`. Every existing authenticator
keeps working without modification.

## Why this exists

The workspace today has three permission primitives in
`starter-spi`:

- `Role` — `Reader | Writer | Admin`. Coarse, hardcoded, three
  values.
- `Scope` — opaque `verb:resource` strings carried on the
  `Principal`, enforced by `require_scope` middleware.
- `require_role` / `require_scope` middleware — boolean gates per
  route.

Those are enough for a single-operator appliance. They fall over the
moment a consumer wants any of:

1. **Per-user CRUD toggles** — "Alice can read flows, Bob can read
   and write, Carol can do everything except delete." Roles are too
   coarse; scopes hardcode the policy into the binary.
2. **Row-level ownership** — "users can edit their own flows but
   not others'." Middleware can't see the resource id; in-handler
   `if principal.subject == flow.owner` checks scatter the rule
   across every CRUD handler and drift.
3. **Extension-defined resources** — `starter-extensions` lets
   consumers add whole new route trees. Today those routes get
   `require_role(Admin)` or nothing. There is no way for an operator
   to grant a specific user access to a single extension's routes
   without giving them `Admin` on everything.
4. **Attribute-driven rules from OAuth** — "GitHub sign-ins from
   `@acme.com` are Writers; anyone else from GitHub is Reader." The
   OAuth crate's `OAUTH_*_ROLE_DOMAIN_MAP` (Phase 4 of the OAuth
   SCOPE) is a one-off shortcut for this single case; a general
   answer is a policy engine that can match on any attribute the
   `Principal` carries.
5. **Policy changes without redeploy** — operators editing CRUD
   permissions in an admin UI, not a `git push`.

This is the problem space `casbin` and similar policy engines are
built for. Starter ships a small, opinionated version of the same
shape — trait seam in `starter-spi`, default RBAC-with-ownership
engine in `starter-authz`, optional Casbin adapter for consumers who
already think in Casbin policies.

## Relationship to existing crates

```
starter-spi                     (Authenticator, Principal, Role, Scope,
   ↑                              + new: PolicyEngine, ResourceRef, Decision,
   │                              + new: ResourceRegistry trait)
   │
   ├── starter-auth-token       (no change; still mints Principal)
   ├── starter-auth-users       (no change; existing middleware keeps working)
   ├── starter-auth-oauth       (small change: stamps oauth.* attrs on Principal.extra)
   │
   ├── starter-authz   ──→ depends on starter-spi only
   │                          (PolicyEngine impls, ResourceRegistry impl,
   │                           require_permission middleware, admin REST routes,
   │                           optional DB-backed policy store)
   │
   ├── starter-server  ──→ optional dep on starter-authz behind a
   │                       cargo feature; mounts the admin policy
   │                       routes and exposes ResourceRegistry to
   │                       extensions at boot
   │
   └── starter-extensions ──→ extensions call
                              `ResourceRegistry::register(spec)` at
                              init; their routes wrap themselves in
                              `require_permission(resource, action)`
```

`starter-authz` is **strictly optional** (workspace R5). Default-
features stay empty; a consumer who doesn't enable it pays nothing —
no policy engine constructed, no admin routes mounted, no extra
migrations. The existing `require_role` / `require_scope` middleware
keeps working untouched as the "no authz crate" baseline.

Auth coupling: `starter-authz` reads the caller's identity through
the `Principal` already produced by whichever `Authenticator` the
binary wires in. It is **not** an auth crate — it does not know how
the caller was authenticated, only what attributes the `Principal`
carries.

## Hard rules (load-bearing)

### R1 — Authz runs after auth, never replaces it

The pipeline is:

```
request → Authenticator → Principal → PolicyEngine::check(p, action, object) → handler
                                       ▲
                                       │
                                       └─ require_permission middleware
                                          OR in-handler engine.check(...)?
```

The `Authenticator` trait, `Principal`, session cookies, bearer
tokens, and OAuth callback are unchanged. The policy engine consumes
a `Principal` it did not produce and emits an `Allow` / `Deny`. A
binary that disables `starter-authz` still authenticates correctly;
it just falls back to the existing `require_role` / `require_scope`
middleware for authorization.

The reason: mixing authentication and authorization in one trait is
the classic mistake that locks you into a single auth strategy.
Keeping them as two stages means a consumer can swap authenticators
(password ↔ OAuth ↔ SAML) without rewriting policies, and swap
policy engines (built-in RBAC ↔ Casbin ↔ custom) without rewriting
authenticators.

### R2 — One trait seam in `starter-spi`; default impl in `starter-authz`

```rust
// in starter-spi
#[async_trait]
pub trait PolicyEngine: Send + Sync {
    /// Decide whether `principal` may perform `action` on `object`.
    /// `object.id == None` is the route-level / collection check
    /// (e.g. "may this user list flows at all?"). `object.id ==
    /// Some(id)` is the row-level check (e.g. "may this user update
    /// flow 42?").
    async fn check(
        &self,
        principal: &Principal,
        action: &str,
        object: &ResourceRef,
    ) -> Decision;
}

pub struct ResourceRef {
    /// Resource kind. Must be registered in the `ResourceRegistry`
    /// (e.g. "flows", "users", "secrets").
    pub kind: String,
    /// Resource id, when the check is for a specific row. `None`
    /// for collection-level / route-level checks.
    pub id: Option<String>,
    /// Subject id of the resource owner, if any. Lets ownership
    /// rules ("owner can update") work without a DB round-trip
    /// from inside the engine.
    pub owner: Option<String>,
}

pub enum Decision {
    Allow,
    Deny { reason: String },
}

pub struct NoopPolicyEngine; // always-Allow; the "authz disabled" baseline
```

`starter-authz` provides the substantive impls:

- `StaticRbacEngine` — RBAC + ownership, policies loaded from
  TOML/YAML at boot. The default. Fits 80% of consumers.
- `DbPolicyEngine` — same model as `StaticRbacEngine` but policies
  live in `starter_authz_policies` (sqlite/postgres) and are edited
  via admin REST routes. Feature-gated; needed when operators must
  change permissions without a redeploy.
- `CasbinEngine` — adapter wrapping `casbin` for consumers who
  already have Casbin policy files. Feature `casbin`; not pulled in
  by default.

A consumer who needs neither RBAC nor Casbin writes their own
`PolicyEngine` impl. The trait is the only public seam.

### R3 — Default-deny on unknown resources; deny-overrides on conflict

When the engine is asked about a `kind` not in the
`ResourceRegistry`, the answer is `Deny { reason:
"unknown_resource" }`. When multiple rules match and disagree, deny
wins. Neither default can be flipped by config — they are the only
defaults compatible with "extensions add routes and forget to
register them."

The reason: default-allow on unknown resources turns a forgotten
`ResourceRegistry::register` call in an extension into a silent
authorization bypass. Default-deny turns the same bug into a loud
`403 unknown_resource` the operator notices on first request. The
trade-off (extension authors must remember to register) is paid
once per extension at init; the alternative trade-off
(authorization bypass) is paid forever.

Deny-overrides on conflict matches Casbin's standard
`priority(p_eft) || deny` model and is the only conflict policy
that doesn't let an over-broad allow rule accidentally widen
access.

### R4 — Resources are registered, not stringly-discovered

Every resource kind a policy can reference must be registered with
the `ResourceRegistry` at boot:

```rust
pub trait ResourceRegistry: Send + Sync {
    fn register(&self, spec: ResourceSpec);
    fn known(&self) -> Vec<ResourceSpec>;
    fn lookup(&self, kind: &str) -> Option<ResourceSpec>;
}

pub struct ResourceSpec {
    /// Stable wire identifier (e.g. "flows", "users").
    pub kind: &'static str,
    /// Actions defined on this resource (e.g. ["read","create","update","delete"]).
    /// Closed list — the admin UI renders exactly these checkboxes.
    pub actions: &'static [&'static str],
    /// Whether rows of this resource have an owner the engine
    /// should consider for ownership rules.
    pub ownership: Ownership,
    /// Human label + description for the admin UI; not consumed by
    /// the engine.
    pub label: &'static str,
    pub description: &'static str,
}

pub enum Ownership {
    /// Rows have no owner concept; ownership rules don't apply.
    None,
    /// Rows have a `subject` owner — the engine can match
    /// `principal.subject == object.owner`.
    Subject,
}
```

Built-in resources (`users`, `sessions`, `tokens`, `oauth_identities`,
`prefs`) are registered by their owning crates at server boot.
Extensions register their own at `ExtensionInit::register_resources`.
Consumer application code registers via the same trait.

The admin UI enumerates `registry.known()` to render the
permissions grid — no hardcoded resource list, no per-extension
admin page, no "where do I add my new route to the permissions
page?" lookup.

### R5 — Two enforcement points: middleware and in-handler

Coarse, route-level checks go through middleware:

```rust
// Route definition
.route("/v1/flows", get(list_flows).layer(require_permission("flows", "read")))
.route("/v1/flows", post(create_flow).layer(require_permission("flows", "create")))
.route("/v1/flows/:id", patch(update_flow).layer(require_permission("flows", "update")))
```

The middleware constructs `ResourceRef { kind: "flows", id: None,
owner: None }` (collection-level) or extracts the `:id` segment for
row-level routes. For row-level checks where the owner matters,
handlers call the engine directly after the load:

```rust
async fn update_flow(State(s): State<AppState>, Path(id): Path<String>,
                     Extension(p): Extension<Principal>, ...) -> Result<...> {
    let flow = s.flows.get(&id).await?;
    s.policy.check(&p, "update", &ResourceRef {
        kind: "flows".into(),
        id: Some(id.clone()),
        owner: Some(flow.owner_id.clone()),
    }).await?;
    // ... proceed
}
```

The middleware is the cheap, declarative path. The in-handler call
is for the cases where the policy needs information that doesn't
exist until after the row is loaded (ownership being the main one).
Both go through the same `PolicyEngine::check`; the policy file
doesn't care which side of the seam the call came from.

The reason: route-level-only is too coarse for ownership;
in-handler-only is too easy to forget. The pattern is "middleware
for the cheap check, in-handler for the row-level refinement" and
matches what every mature authz layer (Django Guardian, Rails
Pundit, Casbin examples) settles on.

### R6 — Policy file format is plain, diffable, and one source of truth

```toml
# starter-authz.toml — loaded by StaticRbacEngine

# Role assignments. A subject can hold multiple roles; rules check
# any-of.
[[assignments]]
subject = "alice@example.com"
roles   = ["editor"]

[[assignments]]
subject = "*@acme.com"        # glob on the email attribute
roles   = ["writer"]

# Rules. evaluated in order; first allow wins, any deny wins overall.
[[rules]]
role     = "editor"
resource = "flows"
actions  = ["read", "create", "update"]
effect   = "allow"

[[rules]]
role     = "*"                # any authenticated user
resource = "flows"
actions  = ["update", "delete"]
condition = "owner"            # principal.subject == object.owner
effect   = "allow"

[[rules]]
role     = "*"
resource = "secrets"
actions  = ["*"]
effect   = "deny"              # explicit deny overrides any allow
```

The DB-backed engine (`DbPolicyEngine`) stores the same shape in
`starter_authz_assignments` and `starter_authz_rules` tables; the
admin REST routes are CRUD over those tables. A consumer can start
with TOML, switch to DB later by importing the file once — no
schema or semantic change.

The reason: a policy that operators have to *understand* during an
incident must be one greppable file or one queryable table, not a
DSL with its own parser and its own opinions. TOML + a documented
rule shape is the smallest thing that works.

### R7 — Built-in roles map to default policies

The three `Role` variants from `starter-spi` (`Reader`, `Writer`,
`Admin`) get a built-in default policy that ships with
`starter-authz` and is loaded **before** any consumer-provided
file:

- `Reader`  → `read` on every registered resource.
- `Writer`  → `read | create | update` on every registered
              resource except `users`, `sessions`, `tokens`,
              `secrets`, and `oauth_identities`; `update` on own
              row for those.
- `Admin`   → `*` on every registered resource.

A consumer's policy file overrides these for any
`(role, resource, action)` triple they care to redefine. The
defaults exist so a binary that enables `starter-authz` but ships
no policy file behaves identically to one using the old
`require_role` middleware — the upgrade is free.

The reason: zero-config upgrades are the only way an additive
crate gets adopted. If turning on `starter-authz` required
hand-writing a 200-line policy file before the binary boots, nobody
would turn it on.

### R8 — `Principal.extra` is the attribute bus; OAuth stamps `oauth.*`

Attribute-driven rules ("Google sign-ins from `@acme.com` get
Writer", "GitHub-linked users can deploy", "verified-email users
can invite") need attributes on the `Principal`. The crate carves
out a reserved namespace inside the existing `Principal.extra`
JSON bag:

```jsonc
{
  "oauth": {
    "provider": "google",            // present only on OAuth sessions
    "provider_sub": "1234567890",
    "email": "alice@acme.com",
    "email_domain": "acme.com",
    "email_verified": true,
    "linked_providers": ["github", "google"]
  },
  "consumer_field": "..."             // consumers keep using extra for their own claims
}
```

`starter-auth-oauth`'s session bridge writes this on session mint;
the password-login path leaves it absent (or writes `oauth: null`
for symmetry). `StaticRbacEngine` exposes attribute matching in
the rule `condition` field:

```toml
[[rules]]
role      = "*"
resource  = "deployments"
actions   = ["create"]
condition = 'oauth.email_domain == "acme.com" and oauth.email_verified'
effect    = "allow"
```

The condition mini-language is **deliberately tiny**: equality,
membership, boolean conjunction. No arithmetic, no string
manipulation, no function calls. Anything more goes through a
custom `PolicyEngine` impl, not condition strings — turning the
policy file into a programming language is how authz crates grow
unmaintainable rule sets.

The reason: putting OAuth attributes on `Principal` (not in a
separate "claims" struct) means every existing piece of code that
takes a `Principal` keeps working; new code that wants attributes
reads them through one well-known JSON path. Reserving `oauth.*`
keeps the namespace from colliding with consumer-defined `extra`
fields.

### R9 — Decisions are observable; denials carry a stable code

Every `Deny` carries a stable `reason` code (`unknown_resource`,
`no_matching_rule`, `explicit_deny`, `not_owner`,
`role_missing`, `attribute_mismatch`) plus a debug-only human
message. The HTTP layer maps `Deny` to `403 Forbidden` with
`{"error": "<reason>"}` — never `500`, never `404`, never a
leaky message.

Every `check` call emits a `tracing` event at `debug` level with
the principal subject, the action, the resource kind+id, the
matched rule id (if any), and the decision. At `info` level only
denials are logged. This is the audit signal the operator greps
during an incident.

The reason: silent denials are the #1 authz support burden.
Stable codes let the UI render "you don't own this flow" vs
"your role doesn't include update on flows" without the server
exposing internal rule ids.

### R10 — Comments explain *why*, never *what*; same R10 as the rest of the workspace

No `// FIXED:` banners, no emoji, no progress logs. Doc-comments
on every public item.

## Repo layout

```
crates/
  starter-spi/                          <- ADD
    src/authz/
      mod.rs                            <- pub use
      engine.rs                         <- PolicyEngine trait, NoopPolicyEngine
      decision.rs                       <- Decision, ResourceRef
      registry.rs                       <- ResourceRegistry trait, ResourceSpec, Ownership
                                            (trait only; impl in starter-authz)

  starter-authz/                        <- NEW. Default-features = [].
    Cargo.toml
    migrations/
      0001_authz_assignments.sql        <- sqlite + postgres variants (feature db)
      0002_authz_rules.sql
    src/
      lib.rs
      registry/
        mod.rs                          <- StaticRegistry impl of ResourceRegistry
                                            (HashMap behind a RwLock; register-once at boot)
      engine/
        mod.rs                          <- pub use
        rbac.rs                         <- StaticRbacEngine: TOML + default-role policies
        rules.rs                        <- Rule, Assignment, condition evaluator
        condition.rs                    <- tiny expression language (eq, in, and/or)
        db.rs                           <- DbPolicyEngine (feature "db")
        casbin.rs                       <- CasbinEngine adapter (feature "casbin")
      middleware.rs                     <- require_permission(kind, action)
      routes/
        router.rs                       <- authz_router::<S>(state) -> Router<S>
        rules.rs                        <- CRUD /v1/authz/rules         (Admin only)
        assignments.rs                  <- CRUD /v1/authz/assignments   (Admin only)
        resources.rs                    <- GET /v1/authz/resources      (registry dump)
        check.rs                        <- POST /v1/authz/check         (dry-run a decision)
      defaults.rs                       <- built-in Reader/Writer/Admin policy
      config.rs                         <- AuthzConfig, file loader
      error.rs
    tests/
      registry_register_once.rs
      rbac_role_matrix.rs
      ownership_condition.rs
      default_deny_unknown_resource.rs
      deny_overrides_allow.rs
      oauth_attribute_rules.rs
      route_middleware_extracts_id.rs
      admin_routes_require_admin.rs
```

## Data model additions

Only `DbPolicyEngine` (feature `db`) touches the database. The
default `StaticRbacEngine` is file + memory only.

```sql
-- migrations/0001_authz_assignments.sql (sqlite)
CREATE TABLE starter_authz_assignments (
  id           TEXT PRIMARY KEY,
  subject      TEXT NOT NULL,           -- exact subject id, or glob ("*@acme.com")
  role         TEXT NOT NULL,           -- role name; matches rules.role
  created_at   TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  created_by   TEXT NOT NULL,           -- admin user id who made the assignment
  UNIQUE (subject, role)
);
CREATE INDEX idx_authz_assignments_subject ON starter_authz_assignments (subject);

-- migrations/0002_authz_rules.sql (sqlite)
CREATE TABLE starter_authz_rules (
  id           TEXT PRIMARY KEY,
  role         TEXT NOT NULL,           -- "*" matches any authenticated principal
  resource     TEXT NOT NULL,           -- kind from ResourceRegistry; "*" matches any
  actions      TEXT NOT NULL,           -- JSON array; ["*"] matches any
  condition    TEXT,                    -- nullable; condition mini-language
  effect       TEXT NOT NULL,           -- "allow" | "deny"
  priority     INTEGER NOT NULL DEFAULT 0,
  created_at   TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  created_by   TEXT NOT NULL
);
CREATE INDEX idx_authz_rules_role_resource ON starter_authz_rules (role, resource);
```

Postgres variants use `TIMESTAMPTZ NOT NULL DEFAULT now()` and
`JSONB` for `actions`, matching the convention in the rest of the
workspace.

Tables are owned by `starter-authz`, not by any auth crate. A
consumer who never enables `starter-authz` never runs these
migrations; a consumer who runs `StaticRbacEngine` (file-only)
never runs them either.

## Routes

All routes require `Admin` role at the existing
`require_role(Admin)` middleware level — authz of authz itself is
deliberately *not* delegated to the policy engine, to avoid the
bootstrap problem ("operator demotes themselves, can no longer fix
it"). The engine still runs after, so additional rules can *further
restrict* admin access but cannot widen it.

| Method | Path                                    | Purpose                                              | Auth         |
| ------ | --------------------------------------- | ---------------------------------------------------- | ------------ |
| GET    | `/v1/authz/resources`                   | Enumerate registered resources + actions             | Admin        |
| GET    | `/v1/authz/rules`                       | List rules                                           | Admin        |
| POST   | `/v1/authz/rules`                       | Create a rule                                        | Admin + CSRF |
| PATCH  | `/v1/authz/rules/{id}`                  | Update a rule                                        | Admin + CSRF |
| DELETE | `/v1/authz/rules/{id}`                  | Delete a rule                                        | Admin + CSRF |
| GET    | `/v1/authz/assignments`                 | List role assignments                                | Admin        |
| POST   | `/v1/authz/assignments`                 | Assign a role to a subject                           | Admin + CSRF |
| DELETE | `/v1/authz/assignments/{id}`            | Revoke an assignment                                 | Admin + CSRF |
| POST   | `/v1/authz/check`                       | Dry-run: would `(subject, action, resource)` allow? | Admin        |

`POST /v1/authz/check` is the "explain this decision" endpoint —
the admin UI uses it to show "Alice would be denied because
`owner != alice` for flow 42." It is also what an external policy
test harness calls in CI.

## Configuration

```
AUTHZ_ENGINE=static                      # static | db | casbin | noop
AUTHZ_POLICY_FILE=./starter-authz.toml   # path; only read when engine=static
AUTHZ_DEFAULT_POLICY=true                # load built-in Reader/Writer/Admin defaults
AUTHZ_DENY_LOG_LEVEL=info                # tracing level for denials
```

Secrets resolution: there are none — policy files and rule tables
are not secrets. (If a consumer's rule references a secret value,
the engine matches on the literal anyway; there is no secret
indirection inside conditions.)

## Extension story

Extensions register their resources in the existing
`ExtensionInit` hook:

```rust
impl Extension for MyExt {
    fn init(&self, cx: &mut ExtensionContext) {
        cx.resources().register(ResourceSpec {
            kind: "myext.widgets",
            actions: &["read", "create", "update", "delete", "publish"],
            ownership: Ownership::Subject,
            label: "Widgets",
            description: "User-authored widgets exposed by myext.",
        });
        cx.routes()
            .route("/myext/widgets", get(list).layer(require_permission("myext.widgets", "read")))
            .route("/myext/widgets/:id", patch(update).layer(require_permission("myext.widgets", "update")));
    }
}
```

The `myext.` namespace prefix is convention, not enforcement —
two extensions registering the same `kind` is a panic at boot
(loud failure beats silent shadowing). The admin UI then shows
"Widgets — read / create / update / delete / publish" in the
permissions grid for every role, identical to a built-in
resource.

## OAuth bridge (one change to `starter-auth-oauth`)

`starter-auth-oauth`'s session-mint path gains a small step: when
the callback resolves a `ProviderIdentity` and mints the
`sas_*` session, it also writes the `oauth.*` block defined in R8
onto the new `Principal.extra` for that session. The password
login path leaves `oauth` absent.

This is the single point of coupling between `starter-authz` and
`starter-auth-oauth`. The OAuth crate doesn't depend on
`starter-authz` (the namespace is just a documented convention);
the authz crate doesn't depend on `starter-auth-oauth` (it reads
JSON, not provider types). Either crate can ship without the
other.

## Flow (a request through the stack)

1. Request arrives. `AuthenticationLayer` runs the configured
   `Authenticator`, produces a `Principal`, sets it in request
   extensions.
2. `require_role(Admin)` (or similar) runs if the route declares
   it — preserves the existing coarse gate.
3. `require_permission("flows", "update")` runs. It constructs
   `ResourceRef { kind: "flows", id: extract_path("id"), owner: None }`
   and calls `policy_engine.check(&principal, "update", &ref)`.
4. Engine resolves the principal's roles (from assignments +
   `Principal.role`), walks matching rules in priority order,
   evaluates conditions against `Principal.extra.oauth.*` and
   `principal.subject == object.owner`, returns `Allow` or `Deny`.
5. On `Allow`, the handler runs. The handler may make an
   additional `engine.check` call for row-level ownership *after*
   loading the row, supplying `owner` this time.
6. On `Deny`, the middleware returns `403 { "error": "<reason>" }`
   and a `tracing::info!` records the denial with structured
   fields.

Every step that fails returns a deliberate, non-leaky response: the
user sees a stable error code; the server logs the underlying
matched/non-matched rule for the operator to grep.

## Testing seams

- `starter-authz::testing::TestEngine` — an in-memory engine
  pre-loaded with a hand-built rule set. Used by tests of
  consumer routes that want to assert "this route is gated by
  permission X" without spinning up the full policy loader.
- `starter-authz::testing::AllowAll` and `DenyAll` — trivial
  `PolicyEngine` impls for unit tests that don't care about
  policy at all.
- `POST /v1/authz/check` doubles as a black-box test endpoint —
  CI can hit it with a fixture set of `(subject, action,
  resource)` triples and assert the decision matrix.

## Smoke tests (before merging)

### "Existing binary keeps working with authz disabled" test

A consumer building a binary with `starter-auth-users` and
**without** `starter-authz` compiles, boots, and serves requests
unchanged. The `require_role` / `require_scope` middleware behaves
exactly as before. No `403 unknown_resource` anywhere. If
`starter-authz` bleeds into the build, workspace R5 has slipped.

### "Default policy gives roles the obvious thing" test

Turn on `starter-authz` with no policy file and `AUTHZ_DEFAULT_POLICY=true`.
A `Reader` can `GET /v1/flows`, cannot `POST /v1/flows`. A
`Writer` can `POST /v1/flows`, cannot `DELETE /v1/users/:id`. An
`Admin` can do both. Equivalent to the previous `require_role`
behaviour — the zero-config upgrade promise.

### "Ownership rule" test

Alice (`Writer`) creates flow `f1`. Bob (`Writer`) tries
`PATCH /v1/flows/f1` and gets `403 not_owner`. Alice tries the
same and succeeds. The rule in the policy file is the one shown
in R6 (`condition = "owner"`).

### "Extension resource registered at boot is enforceable" test

`MyExt` registers `myext.widgets` with actions
`["read","create"]`. An admin assigns a custom role `widgetuser`
to Alice with rule `(widgetuser, myext.widgets, create, allow)`.
Alice hits `POST /myext/widgets` and succeeds. Bob (no rule) gets
`403 no_matching_rule`. If `myext.widgets` were *not* registered,
both would get `403 unknown_resource` — the default-deny check.

### "Two extensions register the same kind" test

Two extensions both register `kind = "widgets"`. The server
**panics at boot** with a message naming both extensions. If the
second registration silently shadowed the first, R4 has slipped.

### "Deny overrides allow" test

A user has role `editor` (allow `flows / update`) and a separate
matching rule denies `flows / update` for `oauth.email_domain ==
"contractor.com"`. The contractor user is denied even though the
allow rule matches. If the order of rule loading flips the
outcome, R3 has slipped.

### "OAuth attributes drive a rule" test

Alice signs in via Google with `email_verified: true,
email_domain: acme.com`. A rule grants `deployments / create` to
`oauth.email_domain == "acme.com" and oauth.email_verified`. The
call succeeds. Bob signs in via Google with
`email_verified: false`; the same call returns `403
attribute_mismatch`. Carol signs in via password (no `oauth`
block); same call returns `403 attribute_mismatch`. The condition
treats missing attributes as not-equal, never as true.

### "Admin cannot lock themselves out" test

The admin authz routes use `require_role(Admin)`, not
`require_permission("authz.rules", ...)`. An admin who writes a
policy file that denies themselves every permission on every
resource can still hit `DELETE /v1/authz/rules/{id}` to roll the
change back. If the recovery path itself goes through the engine,
R bootstrap problem has slipped.

### "Denial logs are greppable" test

Every `Deny` emits exactly one `tracing` event with the documented
structured fields (`subject`, `action`, `kind`, `id?`, `reason`,
`matched_rule?`). The integration-test recording subscriber
asserts the schema on every denial returned across the smoke-test
suite.

### "Dry-run check matches real check" test

For a representative sample of `(principal, action, resource)`
triples, `POST /v1/authz/check` and a real request through the
middleware return the same decision. If they diverge, the admin UI
is lying.

## Non-goals

- **Not authentication.** The crate does not verify credentials,
  does not mint sessions, does not know about cookies or tokens. It
  consumes a `Principal` it did not produce.
- **Not a full policy DSL.** The condition mini-language is
  equality + membership + boolean conjunction. Arithmetic, string
  manipulation, regex, function calls, and recursion are out of
  scope. Consumers who need a real DSL write a custom
  `PolicyEngine` impl (or use the Casbin adapter, which already has
  one).
- **Not OPA / Rego.** Open Policy Agent is the right answer for
  cross-service, network-policy-style authorization. `starter-authz`
  is in-process, per-request, app-level authz. Different problem,
  different answer.
- **Not multi-tenant isolation by itself.** Tenant scoping
  (`Principal.workspace_id`, row `workspace_id`) is a consumer
  concern; the engine can reference `principal.workspace_id ==
  object.workspace_id` in a condition, but the table-level
  isolation (which workspace's rows you can even see) is enforced
  in the query layer, not here.
- **Not field-level authorization.** "Alice can read a user record
  but not their email" is out of scope; that level of filtering
  belongs in the response serialiser, not the gate. A future
  `FieldPolicy` trait could layer on top.
- **Not policy version control.** Rule edits via the admin REST
  routes are logged via `tracing` and recorded in
  `created_at` / `created_by`, but full history (who changed what
  when, with diff) is deferred until `starter-observability` has
  an audit-sink concept. File-based policies version-control
  themselves in git.
- **Not a permission *language* for end users.** The admin UI
  renders the resource × action grid; end users see stable
  `403 <reason>` codes, never raw rule text.
- **Not row encryption / row hiding.** Denial returns `403`, not
  `404`. Consumers who want "you cannot tell this resource exists"
  semantics layer a thin "deny → 404" middleware on specific
  routes themselves; the engine stays honest about what happened.

## Decisions made

- **Trait seam in `starter-spi`, default impl in `starter-authz`.**
  Same pattern as `Authenticator`, `UserStore`, `SessionStore`.
  Keeps `starter-spi` dependency-free and lets consumers swap
  engines.
- **Default-deny on unknown resources; deny-overrides on
  conflict.** The only defaults that don't let a forgotten
  registration become an authorization bypass.
- **RBAC + ownership is the built-in model.** Covers ~80% of
  consumer use cases without an ABAC engine. ABAC-style attribute
  matching is available via the tiny condition language for the
  remaining 20%.
- **TOML policy file + DB-backed table use the same shape.** A
  consumer can start file-based and migrate to DB without a
  semantic change. The admin REST routes operate on the DB shape.
- **Built-in role defaults ship.** Zero-config upgrade from
  `require_role` is the only way the crate gets adopted.
- **`Principal.extra.oauth.*` is the OAuth attribute bus.** Reuses
  the existing extra-claims bag; no schema change to `Principal`.
- **Admin authz routes are role-gated, not permission-gated.**
  Avoids the bootstrap lockout problem. The engine still runs
  *after*, so admins can be further restricted but never further
  empowered through the policy file.
- **Two enforcement points (middleware + in-handler).** Middleware
  is the cheap gate; in-handler is the row-level refinement.
  Both go through the same `check`.
- **Casbin is an optional adapter, not the default.** Bringing
  Casbin into the core would lock the public API to Casbin's
  semantics and pull a heavy dep into every starter binary.
  Optional adapter is the right unit of opt-in.
- **Decisions emit stable `reason` codes.** The UI needs to
  render specific messages; codes let it do that without the
  server leaking rule details.

## Open questions

- **Rule priority interaction with deny-overrides.** Casbin
  resolves "deny wins" *across all matched rules regardless of
  priority*; the design above matches this. A future refinement
  might let `priority` shadow a same-priority deny — but the
  semantics get confusing fast. Defer until a real consumer needs
  it.
- **Negative role assignments ("Alice is *not* `Writer`").**
  Currently roles are additive and removal goes through deletion
  of an assignment row. A negative-assignment shorthand is
  considered if operators report needing it; the rule-level
  `deny` covers the same ground in the meantime.
- **Caching.** `StaticRbacEngine` keeps everything in memory and is
  fast; `DbPolicyEngine` will need an in-process LRU keyed by
  `(subject, role_set, resource, action)` with invalidation on
  rule/assignment writes. Design the invalidation event before
  shipping the cache.
- **Per-workspace policies in a multi-tenant binary.** The
  workspace concept is in `starter-prefs` (`@starter/default`
  sentinel) and elsewhere. Whether rules can be scoped to a
  workspace, or whether `principal.workspace_id` in a condition is
  sufficient, is deferred until a multi-workspace consumer asks.
- **Audit-trail durability.** Rule edits emit `tracing` events
  today. Durable audit (write to an `authz_audit` table or stream
  to `starter-observability`) is deferred until that crate has an
  audit-sink concept.
- **Web UI for the policy grid.** The REST surface exists; the
  React Settings page that consumes it lives in
  `@nube/starter-ui-core` and lands when `starter-authz` is past
  Phase 2.

## Phasing

Each phase is independently mergeable. Stopping after any phase
leaves a working product.

### Phase 1 — `starter-spi` trait surface + `starter-authz` crate scaffold + `StaticRbacEngine`

- `starter-spi::authz` module: `PolicyEngine`, `Decision`,
  `ResourceRef`, `ResourceSpec`, `Ownership`, `ResourceRegistry`,
  `NoopPolicyEngine`.
- `starter-authz` crate with `StaticRegistry`, `StaticRbacEngine`,
  built-in role defaults, TOML loader, condition mini-language.
- `require_permission(kind, action)` axum middleware.
- Built-in resource registrations from `starter-auth-users`
  (`users`, `sessions`, `tokens`), `starter-auth-oauth`
  (`oauth_identities`), `starter-prefs` (`prefs`) gated behind a
  cargo feature on each so they stay opt-in.
- Smoke tests: default-policy-matches-require-role,
  ownership-rule, default-deny-unknown-resource,
  deny-overrides-allow, admin-cannot-lock-themselves-out.

Outcome: a consumer can add `starter-authz`, write a TOML policy,
and per-resource permissions work — including for routes wrapped
with `require_permission`. Coverage of the "tick boxes for CRUD"
ask without any DB schema.

### Phase 2 — `starter-auth-oauth` attribute bridge + condition language attribute support

- Session-mint path in `starter-auth-oauth` stamps the
  `oauth.*` block defined in R8.
- Condition mini-language gains attribute lookup
  (`oauth.email_domain == "acme.com"`).
- Smoke test: oauth-attributes-drive-a-rule.

Outcome: rules can be written against OAuth identity attributes;
the OAuth crate's `OAUTH_*_ROLE_DOMAIN_MAP` shortcut becomes a
generalised condition.

### Phase 3 — `DbPolicyEngine` + admin REST routes

- Migrations `0001_authz_assignments.sql` +
  `0002_authz_rules.sql` (sqlite + postgres variants), feature
  `db`.
- `DbPolicyEngine` impl with in-process LRU + invalidation.
- Admin REST routes (`/v1/authz/rules`, `/v1/authz/assignments`,
  `/v1/authz/resources`, `/v1/authz/check`).
- File→DB importer (`starter-cli authz import --from
  ./starter-authz.toml`).
- Smoke tests: dry-run-matches-real-check,
  admin-routes-require-admin, rule-write-invalidates-cache.

Outcome: operators edit permissions live without redeploy;
existing TOML policies migrate in one command.

### Phase 4 — Extension resource registration helper + admin UI grid

- `ExtensionContext::resources()` accessor.
- React `<PermissionsGrid>` in `@nube/starter-ui-core` that reads
  `GET /v1/authz/resources` and renders the resource × action ×
  role checkbox grid.
- Smoke test: extension-registers-and-is-enforceable
  (already in Phase 1) gains a UI-level test.

Outcome: extension authors get authorization for free, operators
get a single permissions page that lists every extension's
resources without code changes.

### Phase 5 (reserved) — Casbin adapter

- `CasbinEngine` impl wrapping `casbin` crate, feature `casbin`.
- Documentation mapping `starter-authz` rule shape to Casbin
  model files.
- Lands when a consumer arrives with an existing Casbin policy
  they want to keep.

### Phase 6 (reserved) — Field-level policy

- `FieldPolicy` trait + response-serialiser integration.
- "Alice can read users but not their email" cases.
- Lands when a real consumer hits the limit of resource-level
  authz.

## Bottom line

**One new optional crate, one new trait seam in `starter-spi`, one
new `oauth.*` namespace on `Principal.extra`, and one new tower
middleware. RBAC with ownership and attribute conditions covers
the per-user CRUD ticking the existing `Role` enum cannot; default-
deny on unknown resources keeps extension routes safe by
construction; the admin REST surface lets operators edit
permissions live; built-in role defaults mean turning the crate on
is a zero-config upgrade from `require_role`. Casbin is an opt-in
adapter, not the default.**
