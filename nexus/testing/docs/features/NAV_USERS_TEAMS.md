# Feature: Users, Teams, Nav & Access

> Verified: **WORKING end-to-end on nexus-rewrite, 2026-06-11.** Users, teams,
> nav tree, and per-resource grants all run live against the seeded `nexus`
> tenant. Three authz gaps found during this pass were fixed (see "Known issues
> / fixes"). **Status: verified.**

**What we're testing:** CRUD users and teams, build the nav tree, and grant
per-resource access — then prove a non-admin sees only the nav nodes/pages
they've been granted, enforced by RLS + the authz engine.

Architecture recap ([../reference/ARCHITECTURE.md §5](../reference/ARCHITECTURE.md)):
identity from `starter-auth-users` on `/auth/*` + `/v1/tenants/*`; authz via
`starter-authz` `DbPolicyEngine` with `default_policy = true` (admins reach all
via the built-in admin-all rule; non-admins only via explicit grants); RLS
DB-enforced per tenant.

## The access model (read this first — it drives every test below)

**Admins see everything; a non-admin (team/user) sees only what is explicitly
granted to them.** There is no world-default sidebar. Concretely:

- `GET /api/v1/nav` is **access-filtered per node**: a node is returned only if
  the principal holds `view` on it ([routes/nav/list.rs](../../../backend/crates/nexus-api/src/routes/nav/list.rs)).
  An admin matches the built-in admin-all rule, so they see the full tree with no
  per-node grant. A non-admin with no grants gets an **empty sidebar**.
- **Nav nodes and dashboards are separate authz resources.** To make a
  dashboard-backed page actually usable for a non-admin you grant **two** things:
  1. the **nav node** (`nexus.nav_node`) → the page appears in their sidebar, and
  2. the **dashboard** (`nexus.dashboard`) → they can open it (the dashboard list
     + the page route are themselves access-filtered).
  Granting only the nav node makes "Page X" show in the sidebar but open to
  "No dashboards yet" / 403. This is deliberate (one dashboard can mount on
  several nodes, each granted independently), but it means "assign a page" = two
  grants.
- **Subjects** a grant's `role` can name: `team:<slug>` (every member of that
  team), a user's subject id, or `*` (every authenticated tenant member — avoid
  for anything non-public).
- **Static admin pages** (Extensions) are **not** nav-tree nodes — they're
  hardcoded in the UI sidebar and gated client-side with `useCan("admin")`
  ([ui AppSidebar.tsx](../../../ui/src/app/AppSidebar.tsx)). Non-admins don't see
  the link at all; the page also returns "Admin only" as defense in depth.

---

## Exact endpoints (confirmed live — the earlier guesses in this doc were wrong)

Identity / tenant admin (all under `with_role(Admin)` — non-admins get **403**):

| Method | Path | Purpose |
|---|---|---|
| POST | `/v1/tenants/{id}/users` | create a user account + add to tenant in one step |
| GET/POST | `/v1/tenants/{id}/members` | list / add tenant members |
| PATCH/DELETE | `/v1/tenants/{id}/members/{user_id}` | change role / remove member |
| GET/POST | `/v1/tenants/{id}/teams` | list / create teams |
| DELETE | `/v1/tenants/{id}/teams/{team_id}` | delete a team |
| GET/POST | `/v1/tenants/{id}/teams/{team_id}/members` | list / add team members |
| DELETE | `/v1/tenants/{id}/teams/{team_id}/members/{user_id}` | remove team member |

Nav + authz (principal-gated; **create/edit/delete require the relevant grant**):

| Method | Path | Purpose |
|---|---|---|
| GET/POST | `/api/v1/nav` | list (access-filtered) / create a nav node |
| GET/PATCH/DELETE | `/api/v1/nav/{id}` | open / edit / delete a node |
| GET/POST | `/v1/authz/rules` | the raw rule surface — use this to grant **any** kind (incl. `nexus.nav_node`) |
| GET/POST | `/v1/authz/grants` | sugar over `/rules`, but **only** supports `rubix.dashboard.page` in v1 — *not* nav nodes |

> Roles on the wire are `reader | writer | admin` (tenant membership roles).
> The authz action vocabulary for nexus resources is `view | edit | delete`.

---

## Runbook (verified 2026-06-11)

Assumes the stack is up per [../00_setup/QUICKSTART.md](../00_setup/QUICKSTART.md)
and you have an **admin** `$JAR`/`$csrf`/`post()` from the
[cheatsheet](../reference/API_CHEATSHEET.md). `$BASE=http://127.0.0.1:4780`.

### 1. Create a non-admin user + team

```bash
# create the account and add to the nexus tenant as a reader, in one call:
post /v1/tenants/nexus/users \
  '{"email":"viewer1@nexus.local","password":"Granted-Viewer-7x!","role":"reader"}'
# grab the new user's subject id from the members list:
USERSUB=$(curl -s -b "$JAR" $BASE/v1/tenants/nexus/members \
  | python3 -c 'import sys,json;[print(m["user_id"]) for m in json.load(sys.stdin) if m["email"]=="viewer1@nexus.local"]')

# create a team and add the user:
post /v1/tenants/nexus/teams '{"slug":"viewers","display_name":"Viewers"}'
TEAMID=$(curl -s -b "$JAR" $BASE/v1/tenants/nexus/teams \
  | python3 -c 'import sys,json;[print(t["id"]) for t in json.load(sys.stdin) if t["slug"]=="viewers"]')
post /v1/tenants/nexus/teams/$TEAMID/members "{\"user_id\":\"$USERSUB\"}"
```

✅ `GET /api/v1/me` as that user returns `role:"reader"`, `tenant_id:"nexus"`,
`teams:["viewers"]`. Update/remove a member and re-list to confirm it reflects.

### 2. Build a nav tree + a dashboard to mount

```bash
# a dashboard the page will point at:
DASHID=$(post /api/v1/dashboards '{"slug":"viewer-dash","name":"Viewer Dashboard"}' \
  | python3 -c 'import sys,json;print(json.load(sys.stdin)["id"])')

# a granted nav node (mounts the dashboard) and an ungranted control node:
GNODE=$(post /api/v1/nav "{\"title\":\"Granted Page\",\"target\":{\"kind\":\"dashboard\",\"dashboardId\":\"$DASHID\"},\"icon\":\"eye\"}" \
  | python3 -c 'import sys,json;print(json.load(sys.stdin)["id"])')
SNODE=$(post /api/v1/nav '{"title":"Secret Page","target":{"kind":"group"}}' \
  | python3 -c 'import sys,json;print(json.load(sys.stdin)["id"])')
```

✅ `GET /api/v1/nav` as **admin** returns the full tree (these nodes + the seeded
`route` nodes). As the **reader** it's still empty at this point — nothing granted.

### 3. Grant the team access — BOTH the nav node and the dashboard

Nav-node grants must go through `/v1/authz/rules` (the `/grants` sugar refuses
non-dashboard kinds). A `role: "team:<slug>"` rule matches every team member.

```bash
# (a) nav node → the page appears in their sidebar:
post /v1/authz/rules "{\"role\":\"team:viewers\",\"resource\":\"nexus.nav_node\",\"resource_id\":\"$GNODE\",\"actions\":[\"view\"],\"effect\":\"allow\",\"priority\":100,\"tenant_id\":\"nexus\"}"
# (b) dashboard → they can actually open it:
post /v1/authz/rules "{\"role\":\"team:viewers\",\"resource\":\"nexus.dashboard\",\"resource_id\":\"$DASHID\",\"actions\":[\"view\"],\"effect\":\"allow\",\"priority\":100,\"tenant_id\":\"nexus\"}"
```

> Skipping (b) is the classic trap: the sidebar shows "Granted Page" but it opens
> to "No dashboards yet" because `GET /api/v1/dashboards` (access-filtered)
> returns nothing and the page route is 403.

### 4. Prove it as the non-admin (the negative cases are the point)

Log in with a **fresh cookie jar** as `viewer1@nexus.local`, then:

```bash
UJAR=$(mktemp)
curl -s -c "$UJAR" -X POST $BASE/auth/login -H content-type:application/json \
  -d '{"email":"viewer1@nexus.local","password":"Granted-Viewer-7x!"}' >/dev/null

# sidebar = ONLY the granted node:
curl -s -b "$UJAR" $BASE/api/v1/nav | python3 -c 'import sys,json;print([n["title"] for n in json.load(sys.stdin)])'
# → ['Granted Page']

# the granted page opens, the ungranted one is denied:
curl -s -o /dev/null -w "granted node %{http_code}\n" -b "$UJAR" $BASE/api/v1/nav/$GNODE   # 200
curl -s -o /dev/null -w "secret  node %{http_code}\n" -b "$UJAR" $BASE/api/v1/nav/$SNODE   # 403

# the dashboard is visible + openable:
curl -s -b "$UJAR" $BASE/api/v1/dashboards | python3 -c 'import sys,json;print([d["slug"] for d in json.load(sys.stdin)])'  # ['viewer-dash']
curl -s -o /dev/null -w "open dash %{http_code}\n" -b "$UJAR" $BASE/api/v1/dashboards/viewer-dash  # 200
```

✅ Observed (2026-06-11): nav `['Granted Page']`; granted node 200, secret node
403; dashboards `['viewer-dash']`; open 200.

### 5. Negative tests on the admin surface + mutations (must all be 403)

```bash
g(){ printf "%-40s " "$2"; curl -s -o /dev/null -w "%{http_code} (want 403)\n" -b "$UJAR" $BASE$1; }
g /v1/tenants                       "reader list tenants"
g /v1/tenants/nexus/members         "reader list members"
# reader cannot CREATE nav nodes or dashboards:
curl -s -o /dev/null -w "reader POST /api/v1/nav        -> %{http_code} (want 403)\n" \
  -b "$UJAR" -X POST $BASE/api/v1/nav -H content-type:application/json \
  -H "X-CSRF-Token: $ucsrf" -d '{"title":"x","target":{"kind":"group"}}'
curl -s -o /dev/null -w "reader POST /api/v1/dashboards -> %{http_code} (want 403)\n" \
  -b "$UJAR" -X POST $BASE/api/v1/dashboards -H content-type:application/json \
  -H "X-CSRF-Token: $ucsrf" -d '{"slug":"x","name":"x"}'
```

✅ All 403 (these were 200 before the fixes below).

### 6. Revoke

Delete the rule via `DELETE /v1/authz/rules/{id}` (or remove the team member) and
re-run step 4 — the node/dashboard disappear from the reader's views.

---

## Acceptance criteria

- ✅ Admin sees the full nav tree (built-in admin-all rule, no grant needed).
- ✅ A fresh non-admin with **no** grants sees an **empty** sidebar and is denied
  every nav node + page.
- ✅ A single nav-node grant exposes exactly that node; the dashboard needs its
  **own** grant before the page opens.
- ✅ Team grant ⇒ all team members inherit it; removing a member removes access.
- ✅ Cross-tenant isolation: a user in tenant B cannot see/grant tenant A's
  resources (RLS + tenant scoping). A cross-tenant dashboard mount is rejected
  (`routes_nav_e2e::a_cross_tenant_dashboard_target_is_rejected`).
- ✅ Non-admins cannot **create** nav nodes/dashboards or touch `/v1/tenants/*`.
- ✅ Static admin pages (Extensions) are hidden from non-admins in the sidebar.

---

## The negative tests matter most here

Access control is only proven by the **denied** cases. For every "user X can see
Y", also assert "user X cannot see Z". Capture both in the scenario.

---

## Known issues / fixes

- ✅ **Fixed 2026-06-11 — `POST /api/v1/nav` and `POST /api/v1/dashboards` had no
  authz check.** A reader could create nav nodes / dashboards (200). Both now run
  a kind-wide `edit` check (`authz::require_create`) — admins allowed, non-admins
  403. Regression: `routes_nav_e2e::creating_a_nav_node_requires_a_kind_wide_grant`.
- ✅ **Fixed 2026-06-11 — `/v1/tenants/*` was mounted with no role gate.** Any
  authenticated reader could list tenants and CRUD members/teams/users. Now
  wrapped in `with_role(Admin)` in `nexus-api/src/identity.rs`.
- ✅ **Fixed 2026-06-11 — seed over-shared the default sidebar.** `seed-admin`
  wrote a `role:"*"` view grant on all 10 default nav nodes (incl. admin-only
  Access/Audit), making the whole sidebar world-visible. Removed — default nodes
  are seeded structurally only; admins see them via the admin rule, non-admins by
  explicit grant. (Re-running an old seed won't re-add the `*` grants.)
- ✅ **Fixed 2026-06-11 — "Extensions" sidebar link was always rendered.** It's a
  static UI link, not a nav-tree node, so server filtering didn't touch it. Now
  gated with `useCan("admin")` in the UI.
- ⚠️ **Two grants per assigned page.** Granting a dashboard-backed nav node does
  **not** grant the dashboard; you must grant both (`nexus.nav_node` +
  `nexus.dashboard`). If the Access UI doesn't yet do both in one share action,
  the page will look broken ("No dashboards yet") despite the sidebar entry.
