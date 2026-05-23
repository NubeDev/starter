# AUTH — sessions, tokens, Studio→agent, and how AuthZ composes

> Source: `rubix/SCOPE.md` §"Identity, sessions, and Studio→agent
> auth", "Decisions made" (AuthZ + Zitadel + auth/oauth bullets), R2
> (sessions are NOT nodes), R3 (one API, no back channels), R5
> (`Principal` re-used from `starter-spi`). Cross-refs: `MIGRATIONS.md`
> (the `starter_auth_*` migration sources), the Phase 7 audit-log
> work already landed in this repo (`DOCS/auth/authz/`).

This doc is the contract for **how a Studio click becomes an
authenticated, authorised slot write**, across the four transports
rubix supports (REST + SSE, gRPC, MCP, CLI), plus how the in-tree
extension supervisor injects credentials into block processes.

## The principle

- **Identity** is owned by `starter-auth-users` (local users +
  sessions + bearer tokens + tenants + teams) and
  `starter-auth-oauth` (GitHub / Google federation; Zitadel hookup
  documented here, swapped behind the `Authenticator` trait).
- **Authorisation** is owned by `starter-authz` — the Phase 7 surface
  (tenants, teams, decision audit, extension `permission:` manifests)
  is binding. Rubix consumes it; no parallel RBAC.
- **All four authentication paths resolve to the same `Principal`
  (`starter-spi`)** before hitting domain code. **No domain function
  inspects how the principal authenticated.** That is the seam that
  lets us add Zitadel without rewriting `domain-*`.

Sessions are **not nodes** (R2's load-bearing test: nobody outside
the auth subsystem needs to read a session token). Users, tenants,
and teams **are** nodes (Studio renders them, flows branch on them,
MCP queries them) — see `EVERYTHING-AS-NODE.md`.

## Studio→agent auth — the four paths

```text
                  ┌──────────────────────────┐
                  │       Principal          │  ← all four paths land here
                  │ (starter-spi)            │     before domain code runs
                  └────────────▲─────────────┘
                               │ resolved by Authenticator trait
       ┌───────────────┬───────┴────────┬────────────────┐
       │               │                │                │
  cookie session   API bearer       MCP bearer       local token
   (REST + SSE)    (gRPC)           (stdio + HTTP)   (CLI)
       │               │                │                │
       ▼               ▼                ▼                ▼
 starter-auth-users  starter_auth_users_tokens   keyring / env var
 sessions table     (long-lived hashed)         (RUBIX_TOKEN)
```

### REST + SSE — cookie session

- Login: `POST /auth/login` (provided by `starter-auth-users`) mints a
  session and sets an HttpOnly Secure SameSite=Strict cookie.
- The cookie carries an opaque session id; `starter-auth-users`
  resolves it to a `Principal` on every request via the
  `Authenticator` extractor.
- Logout: `POST /auth/logout` deletes the session row and clears the
  cookie.
- Studio reads/writes through `@rubix/ui-core`'s `AuthProvider` —
  the single React seam. `AuthProvider` exposes `useAuth()` returning
  `{ principal, status, login, logout }`. Pages and blocks never
  parse cookies directly.
- SSE inherits the cookie because it's an HTTP request; reconnects
  re-authenticate via the same cookie.

### gRPC — long-lived bearer

- `Authorization: Bearer <api-token>` header on the gRPC channel.
  The token is minted from `starter_auth_users_tokens` (hashed at
  rest), scoped per `Principal`.
- Tonic interceptor resolves the bearer into a `Principal` and
  stuffs it into request extensions; the handler reads it via the
  same `Authenticator` shape as REST.
- Tokens rotate via `POST /auth/tokens/rotate`; revocation is row
  deletion. Studio surfaces a tokens-management page in Phase 1.

### MCP — long-lived bearer

- Same `Authorization: Bearer <api-token>` shape. Stdio MCP injects
  the bearer via env var (`RUBIX_MCP_TOKEN`) at process spawn; HTTP
  MCP carries it as a header. Both resolve through the same
  `Authenticator` and surface the same `Principal`.
- The `starter-ext-mcp` adapter (already shipped in this repo) calls
  `engine.check()` at tool-dispatch using `AuthGate.permission` — the
  shared field consumed uniformly by REST, MCP, and gRPC adapters.
  Manifest-declared `permission: { resource, action }` is enforced
  per tool call. See the Phase 7 worked example in
  `examples/authz-demo/`.

### CLI — local token

- On a dev machine: `agent login` performs an interactive login,
  caches the resulting token in the OS keyring via
  `starter-secrets-keyring`. Subsequent `agent <cmd>` calls read it
  from the keyring.
- In CI: env var `RUBIX_TOKEN`. No keyring; no interactive prompt.
- The CLI **never hits HTTP directly** — it goes through
  `rubix-agent-client`, which adds the bearer header.

### Block process (Rust extension) — supervisor-injected bearer

- The extension supervisor (consumes `starter-extensions`) mints a
  per-block bearer token at spawn time and injects it via env var
  into the child process.
- The block reads the bearer through `rubix-extensions-sdk`'s `Ctx`
  — never the raw env var. The SDK owns the resolution so a future
  switch from env-var injection to a Unix socket or shared-memory
  handoff doesn't touch block code.
- The token is **block-scoped**: its `Principal` carries the block
  id, and `starter-authz` rules can grant or deny per block id. The
  block manifest declares the permissions it needs
  (`permission: { resource: …, action: … }` per Phase 7); the
  supervisor refuses to start a block whose required permissions
  haven't been granted to its bearer.

## Resolving to a `Principal`

`starter-spi::Principal` carries:

```rust
pub struct Principal {
    pub id:        PrincipalId,           // stable user / token id
    pub kind:      PrincipalKind,         // User | Token | Block | System
    pub tenant_id: Option<TenantId>,      // None for System
    pub teams:     SmallVec<[TeamId; 4]>, // membership; drives rule grammar
    pub scopes:    SmallVec<[Scope; 8]>,  // narrowed capabilities
}
```

The `Authenticator` trait — implemented per transport — returns a
`Principal` (or a 401). Once resolved, the principal flows as a
typed parameter into every domain function. **No domain function
reads cookies, parses bearers, or looks at HTTP headers.** That is
the seam (R4) that keeps the layer arrow intact.

`PrincipalKind::Block` is the rubix-specific addition; the rest map
1:1 to the `starter-auth-users` Phase 7 schema already in the tree.

## How AuthZ composes (the layer order)

The Phase 7 work already landed: `with_role` (outer) →
`with_scope` → `with_permission` (inner) → `handler`. The
documented rationale (and the audit consequence) lives in
`starter-ext-spi`'s adapter docs; this section restates the rubix-
specific implications.

- **`with_role`** — coarse-grained guard (`require_role`,
  `require_scope`) wired at route registration time. A role deny
  short-circuits before `with_permission` ever runs; the audit log
  records the role deny via `tracing`, not via the permission audit
  table. Dashboards that aggregate "permission denies" must exclude
  pre-role rejections.
- **`with_scope`** — bearer-token scope narrowing. A token minted
  with `scopes: ["points:read"]` cannot reach a `points:write`
  handler regardless of the underlying user's role.
- **`with_permission`** — fine-grained, manifest-declared via
  `KindManifest.permissions`. The Phase 7 audit log records every
  deny here. Resource/action validated at build time against
  `ResourceRegistry::lookup`; an unknown resource in a manifest
  makes the extension mount fail (the rest of the host still comes
  up) — this is the `unknown-resource-is-build-error` smoke test
  already passing in the tree.

This order is **documented in code and in the wiring point** as a
doc comment (per the Phase 7 stage-4 work). Do **not** flip the
order to "fix" the audit-coverage gap — that breaks the role-as-
short-circuit guarantee and is the wrong trade.

## Tenants and teams — how rubix composes with Phase 7

- A **tenant** owns a slice of the graph. Devices, points, schedules,
  alarms, dashboards all have a `tenant_id` slot (or a containing
  ancestor whose slot supplies it). Default-deny is cross-tenant:
  the `with_permission` middleware fails closed if the requesting
  principal's `tenant_id` doesn't match the resource's `tenant_id`.
- A **team** is an additional permission grant. The rule grammar
  `principal.teams contains "ops"` (already in `starter-authz`)
  composes with the tenant check. Use teams for cross-cutting roles
  (operators, integrators, viewers) rather than copying permission
  grants per user.
- The `users`, `tenants`, and `teams` themselves are **nodes** in
  rubix's graph (per R2). Studio's user-admin page reads + writes
  via the slot API; flows can branch on team membership; MCP can
  query the user list. The underlying tables live in
  `starter-auth-users` (migration source `starter_auth_users`); the
  rubix-side node wrappers live in a domain crate (Phase 1 wires
  this up).

## Zitadel hookup

`rubix` is **not an SSO provider**. The OIDC seam is the
`Authenticator` trait in `starter-spi`. To wire Zitadel:

1. Implement `Authenticator` for Zitadel (validates the IdP token,
   looks up or provisions the local user, returns the `Principal`).
2. Mount the impl behind `Authenticator` in the agent binary
   (`apps/agent/src/main.rs`); the rest of the stack does not
   change. REST, gRPC, MCP, CLI continue to surface the same
   `Principal`.
3. Studio's `AuthProvider` swaps its login UI for a redirect to the
   Zitadel hosted login; the cookie flow is unchanged.
4. **No domain crate is touched.** That is the R4 + R5 payoff.

The same seam covers GitHub / Google federation (already provided by
`starter-auth-oauth`) and any future OIDC provider.

## Secrets (cross-link)

Auth tokens, OAuth client secrets, OIDC issuer keys, block bearers
— all flow through `starter-secrets-*` (file in cloud, keyring on
dev/desktop). The `SecretStore` trait is the only surface a
`domain-*` crate sees. Block authors get `Ctx::secret(name)` from
`rubix-extensions-sdk`, never the raw store. Detail:
`docs/design/SECRETS.md` lands before the phase that needs it (the
SCOPE marks it as just-in-time, not Phase 0).

## Audit log

Phase 7 ships `starter_authz_decisions`. Every `with_permission`
deny lands there with the principal, resource, action, the rule
that matched, and the `surface` field (`rest | mcp | grpc`). The
audit log is queryable per tenant; Studio's audit page (Phase 4
hardening) renders it.

Role denies and scope denies are **not** recorded in the audit table
— they short-circuit upstream and emit `tracing` events. Dashboards
that present "permission denies" filter only on the audit table;
dashboards that present "auth events" union both surfaces.

## Smoke tests (already green in this repo)

The Phase 7 work landed these. They apply unchanged in rubix:

- `per-entry-permission-applied` — manifest `permission:` is
  enforced per route entry.
- `unknown-resource-is-build-error` — an extension with an unknown
  resource kind refuses to mount; the rest of the host comes up.
- `role+permission-compose-correctly` — role deny short-circuits;
  the permission audit table records nothing for the role deny.
- `mcp-permission-applied`, `grpc-permission-applied` — parity
  across MCP and gRPC.
- `surface-decisions-share-audit-trail` — a deny via REST, MCP,
  and gRPC each lands in the audit table with the right `surface`
  field, distinguishable by dashboards.

A rubix domain crate that adds a kind with `permissions` declared
gets the above for free. Adding a `permission` to a kind manifest
without seeding authz grants for pre-existing principals **breaks
them silently at the next deploy** — see `KIND-MANIFEST.md`
"Permission added without an authz seed" pitfall.

## Phase 1 entry expectation

For Phase 1 (devices + points + Studio shell), this doc plus the
existing Phase 7 audit work means:

- Studio's login → `AuthProvider` → cookie-session flow works against
  `starter-auth-users` as-shipped.
- `domain-devices` and `domain-points` declare `permissions` in
  their `KindManifest`s; the audit log records denies.
- The CLI's `agent login` writes a token to the keyring; subsequent
  CLI calls succeed.
- A second tenant cannot read the first tenant's devices (the
  "Build a new UI" smoke test exercises this via two Studio sessions
  against the same agent).

Zitadel + the mobile-admin (Dart) auth path are documented above but
not exercised until Phase 5. The seam is fixed at Phase 0; the
implementations can land later without touching `domain-*`.

## What this doc deliberately does NOT cover

- **Password reset flows** — owned by `starter-auth-users`. Studio
  links to its hosted pages.
- **MFA / WebAuthn** — same. The `Authenticator` returns a
  `Principal` regardless of the second-factor path.
- **Cookie-only-vs-bearer policy debates** — settled: cookies for
  REST + SSE, bearers for gRPC + MCP + CLI. No mixing.
- **`SecretStore` impls** — covered in the just-in-time
  `SECRETS.md`.
