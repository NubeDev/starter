# AUTH — sessions, OAuth, authz wiring + the R3 identity carve-out

> Cites: SCOPE [R3](../../SCOPE.md#r3), [R4](../../SCOPE.md#r4).
> **Phase 2a entry gate.**

## What rubix consumes

- `starter-auth-users` — local users, sessions, tokens.
- `starter-auth-oauth` — GitHub / Google OAuth callbacks.
- `starter-authz` — tenants + teams + policy + decision audit.

Rubix is multi-user from day one; `starter-auth-token` (single-owner
bearer) is **not** used — see SCOPE Non-goals.

## The R3 identity carve-out

Per R3, identity is **not a slot**. Sessions, tokens, OAuth flow
state, tenant + team membership, authz decisions live in the auth
crates' own tables — not the slot store. Reason: the slot store is
gated by authz; making it the home of identity is circular.

Identity is exposed *read-only* via **system kinds** whose slot
reads delegate to the auth crates:

| System kind | Reads via | Purpose |
|---|---|---|
| `sys.identity.session` | `starter-auth-users` | who is calling |
| `sys.identity.tenant` | `starter-authz` | which tenant they're in |
| `sys.identity.team` | `starter-authz` | which team(s) they belong to |
| `sys.identity.preferences` | `starter-prefs` | their units / time / locale (R6) |

Writes go through the auth crates' own APIs, never through the slot
write API.

## Authentication on the MCP endpoint

Phase 2a exit: an unauthenticated MCP request returns 401. The
Authenticator shape for MCP-transport-level auth (Claude Desktop →
rubix) is an **upstream gap to resolve** — see
[STARTER-CHANGES.md](./STARTER-CHANGES.md) "Phase 2a (gates)".

## Authorization

`starter-authz` Phase 7 gates every REST + gRPC + MCP + CLI route.
Decision audit lands in the configured audit sink. Per-tenant
filtering is applied at the query layer, never client-side.

### Per-verb permission declaration

Every rubix tool declares its required `starter-authz` permission
string as a `pub const REQUIRED_PERMISSION: &str` next to its
`DESCRIPTOR` in the verb's DTO file (e.g.
`rubix-spi/src/dto/system/disk.rs`). The dispatch wrapper reads it
and calls `Authz::check` before invoking the underlying probe. This
keeps each verb file the single source of truth for everything
about that verb: the DTOs, the descriptor, the thresholds, and the
permission. No central permission table to drift.

Permission strings used so far:

| Permission | Verbs |
|---|---|
| `system.read` | `rubix.system.disk`, `rubix.system.db`, `rubix.system.flow_errors` |
| `system.alert` | `rubix.alert.send` |

Write-side verbs ride on a different permission from read-side
verbs so an operator can be granted observation without alerting.
New permission strings land alongside the first verb that needs
them and get a row in the table above in the same PR.

## OAuth callback flow

`starter-auth-oauth` provides the callback handlers; rubix mounts
them under `/auth/oauth/...`. Provider config (GitHub, Google) is
loaded from the secrets store at boot.

## Bootstrap-user CLI

First-run admin creation is an out-of-band operator action, never a
public HTTP route. The `rubix-admin` binary (a sibling `[[bin]]` in
the `rubix-agent` crate — the six-crate ceiling stays intact) exposes
a `bootstrap-user` subcommand:

```
rubix-admin bootstrap-user --email <e> --password <p>
```

Both flags fall back to `RUBIX_BOOTSTRAP_EMAIL` /
`RUBIX_BOOTSTRAP_PASSWORD` env vars; CLI wins when both are present.
`RUBIX_DSN` is mandatory — the command fails fast when unset rather
than silently no-op'ing.

The subcommand:

1. Opens a Postgres pool via `starter_store_postgres::pool::connect`.
2. Applies `starter_auth_users::migration::postgres_migration_source()`
   so the `starter_auth_users_*` tables exist on a fresh DB.
3. Calls `starter_auth_users::admin::create_admin` with
   `Role::Admin`.

Idempotency rule: `AdminError::Conflict` is treated as success **only**
when the existing row has the same email and the admin role. A
different role is a hard error — the CLI refuses to silently
escalate or demote an existing account.

The `mani run bootstrap-user` task in [../../MANI.md](../../MANI.md)
wraps this subcommand with the standard env-var contract; the
migration source it applies is the same one chained into the agent's
boot plan per [../migrations/README.md](../migrations/README.md).

## Phase 2a entry gates

Both `AUTH.md` and `MIGRATIONS.md` must be **finalized** before
Phase 2a code lands. Migration order is the load-bearing dependency:
starter migrations run first, rubix migrations after — but the
ordering must be deterministic and testable.
