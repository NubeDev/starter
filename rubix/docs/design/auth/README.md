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

## OAuth callback flow

`starter-auth-oauth` provides the callback handlers; rubix mounts
them under `/auth/oauth/...`. Provider config (GitHub, Google) is
loaded from the secrets store at boot.

## Phase 2a entry gates

Both `AUTH.md` and `MIGRATIONS.md` must be **finalized** before
Phase 2a code lands. Migration order is the load-bearing dependency:
starter migrations run first, rubix migrations after — but the
ordering must be deterministic and testable.
