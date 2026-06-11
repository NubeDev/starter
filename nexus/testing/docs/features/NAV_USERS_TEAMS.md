# Feature: Users, Teams, Nav & Access

> Verified: nexus-rewrite tip on 2026-06-10. **Status: scaffold.**

**What we're testing:** CRUD users and teams, build the nav tree, and grant
per-node access — then prove a non-admin user sees only the nav nodes/pages
they've been granted, enforced by RLS + the authz engine.

Architecture recap ([../reference/ARCHITECTURE.md §5](../reference/ARCHITECTURE.md)):
identity from `starter-auth-users` on `/auth/*`; authz via `starter-authz`
`DbPolicyEngine` with `default_policy = true` (admins reach all; non-admins only
via explicit grants); **nav nodes are the grant unit** (`nexus.nav_node`);
RLS DB-enforced per tenant.

---

## Runbook (fill in as built)

### 1. Users & teams CRUD

1. [ ] Create a second (non-admin) user in the admin tenant (`/auth/tenants/{id}/users`).
2. [ ] Create a team; add the user to it.
3. [ ] ✅ `GET /api/v1/me` as that user returns the right tenant + teams.
4. [ ] Update + remove a member; ✅ membership reflects.

> Confirm exact user-create / team endpoints in `backend/openapi.json` under
> `/auth/*` — `starter-auth-users` owns them; fill the precise paths here.

### 2. Nav tree CRUD

1. [ ] Build a small tree: a `group` header with `dashboard` and `route` children
       (`POST /api/v1/nav`).
2. [ ] Reorder via `sort_order`; reparent via `parent_id` (`PUT /api/v1/nav/{id}`).
3. [ ] ✅ `GET /api/v1/nav` returns the nested tree in order.
4. [ ] Delete a node; ✅ children handled per spec.

### 3. Per-node access grants

1. [ ] As admin, grant the non-admin user (or their team) `viewer` on one nav node
       (`POST /v1/authz/resources/nexus.nav_node/{node_id}`).
2. [ ] ✅ As the non-admin: `GET /api/v1/nav` returns **only** granted nodes; the
       granted page opens; a non-granted page/node is denied (403 / not listed).
3. [ ] Grant `editor`/`manager`; ✅ the user can now edit/manage as the role allows.
4. [ ] Revoke; ✅ access disappears.

---

## Acceptance criteria

- ✅ Admin sees the full nav tree (default_policy = true).
- ✅ A fresh non-admin with **no** grants sees an empty/limited nav and is denied
  every nav node + page.
- ✅ A single node grant exposes exactly that node + its page, nothing else.
- ✅ Team grant ⇒ all team members inherit it; removing a member removes access.
- ✅ Cross-tenant isolation: a user in tenant B cannot see/grant tenant A's nodes
  (RLS + tenant scoping).
- ✅ Roles are honored: `viewer` can't edit; `editor`/`manager` can per spec.

---

## The negative tests matter most here

Access control is only proven by the **denied** cases. For every "user X can see
Y", also assert "user X cannot see Z". Capture both in the scenario.

---

## Known issues / fixes

- _record fixes here_
