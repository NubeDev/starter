# Access Control redesign — IA + UX proposal

Status: draft for review
Owner: ap@nube-io.com
Scope: frontend only (`/admin/access`, `/admin/users`, `@nube/starter-ui-authz`). No backend API changes assumed; one new read endpoint is suggested as optional.

The current `/admin/access` page is a flat tab strip over eight unrelated panels, and `/admin/users` lives on a separate route. There is no logical flow from "I want to give user X access to tenant Y in team Z" to the screens that let you do it. This proposal merges Users into Access Control and reorganises the page around a tenant -> team -> user hierarchy, with rules / assignments / resources / check / decisions repositioned as scoped tools instead of top-level siblings.

Screenshots of every current tab were captured via Playwright against the running dev server at `127.0.0.1:5173` and are referenced inline (see [`/tmp/ac-screens/`](file:///tmp/ac-screens/)).

---

## 0. Backend gap — the UI cannot work today

Before any of the IA / UX changes below land, the backend has to actually expose the routes the panels call. Verified against the running agent on `127.0.0.1:8088` on 2026-05-29:

```
$ curl -s http://127.0.0.1:8088/openapi.json | jq '.paths | keys'
[
  "/api/v1/auth/login",
  "/api/v1/auth/logout",
  "/api/v1/auth/me",
  "/api/v1/auth/token",
  "/livez",
  "/healthz"
]

$ curl -i http://127.0.0.1:8088/v1/tenants            # 404
$ curl -i http://127.0.0.1:8088/v1/authz/decisions    # 404
```

Six routes total. Neither `/v1/tenants*` nor `/v1/authz/*` is mounted. Every panel that calls them gets a 404 directly from the agent — this surfaces in the browser as the `127.0.0.1:5173` errors in the proposal preamble, but the Vite proxy is innocent (it forwards to 8088, which returns the 404).

### 0.1 What exists in the crates but is unmounted

- [`crates/starter-auth-users/src/routes/tenants.rs#L121`](../../../crates/starter-auth-users/src/routes/tenants.rs#L121) — `tenants_router(Arc<dyn TenantStore>)` defines:
  - `POST/GET /v1/tenants`, `GET/PATCH /v1/tenants/{id}`
  - `POST/PATCH/DELETE /v1/tenants/{id}/members[...]`
  - `POST/DELETE /v1/tenants/{id}/teams[/{team_id}]`
  - `POST/DELETE /v1/tenants/{id}/teams/{team_id}/members[/{user_id}]`
- [`crates/starter-authz/src/routes/router.rs#L25`](../../../crates/starter-authz/src/routes/router.rs#L25) — `authz_router(AuthzRoutesState)` defines:
  - `GET/POST /v1/authz/rules`, `PUT/DELETE /v1/authz/rules/{id}`
  - `GET/POST /v1/authz/assignments`, `DELETE /v1/authz/assignments/{id}`
  - `GET /v1/authz/resources`
  - `POST /v1/authz/check`
  - `GET /v1/authz/decisions` (only when `AuthzRoutesState.decision_sink = Some(_)`)

Neither is referenced anywhere under `rubix/crates/rubix-agent/`.

### 0.2 What the agent runs today

The agent's router is composed in [`rubix/crates/rubix-agent/src/bin/rubix_agent/compose.rs`](../../crates/rubix-agent/src/bin/rubix_agent/compose.rs). Around [compose.rs#L454-L456](../../crates/rubix-agent/src/bin/rubix_agent/compose.rs#L454-L456) it mounts `auth_routes` under `/api/v1` and gates the tools router. It never calls `tenants_router(...)` or `authz_router(...)`.

The authz engine the tools-router gate consults is built by [`boot/authz.rs#L23`](../../crates/rubix-agent/src/boot/authz.rs#L23) — it constructs a `StaticRbacEngine` from an empty `AuthzConfig::default()` (so `default_policy = true` — every authenticated principal is allowed). That engine is returned as `Arc<dyn PolicyEngine>`, but `authz_router` requires the concrete `Arc<DbPolicyEngine>`. So mounting authz requires swapping the engine builder, not just adding a `.merge(...)` call.

### 0.3 Required backend changes

These are blockers for any of Sections 2–6 to be testable in a browser:

1. **Mount `tenants_router`** in `compose.rs`, behind the same `with_principal(auth.authenticator)` gate the admin routes use:
   - Construct `let tenants = Arc::new(PgTenantStore::new(pool.clone()))` (the impl already exists at [`crates/starter-auth-users/src/store/tenant_store/postgres.rs#L28`](../../../crates/starter-auth-users/src/store/tenant_store/postgres.rs#L28)).
   - `app = app.merge_external(Router::new().merge(tenants_router::<()>(tenants)).layer(with_principal(...)))`.
   - The `0005_tenants.sql` / `0006_teams.sql` migrations are already chained via `postgres_migration_source()` in [`boot/migrations.rs#L67`](../../crates/rubix-agent/src/boot/migrations.rs#L67) — no migration work needed.

2. **Swap the authz engine to `DbPolicyEngine`** in [`boot/authz.rs`](../../crates/rubix-agent/src/boot/authz.rs):
   - Return `Arc<DbPolicyEngine>` (concrete) instead of `Arc<dyn PolicyEngine>` so the same handle can feed both the tools-router gate and `AuthzRoutesState`. (The gate consumes `PolicyEngine`; `DbPolicyEngine` implements it.)
   - Construct from the live pool, not from `AuthzConfig::default()`.
   - Decision point: the gate flips from "allow every authenticated user" to "consult the rules table". An empty table = deny-by-default if `default_policy = false`. **Seed a bootstrap rule** in the same migration that creates the tables: `Role::Admin` → `*` → `allow`. Without this, the first request after the change locks the operator out of the very UI that lets them write rules.

3. **Add the authz migration source** to the chain in `boot/migrations.rs` (verify `starter-authz` exposes a `postgres_migration_source()` equivalent; if not, add one). The chain must run *before* `RUBIX_TENANTS_MIGRATION_SOURCE` only if authz depends on rubix-tenants — verify FK order before placing it.

4. **Mount `authz_router`** in `compose.rs`:
   - Build `AuthzRoutesState { engine, registry, decision_sink: Some(Arc::new(DbDecisionSink::new(pool.clone()))) }`.
   - `app = app.merge_external(Router::new().merge(authz_router::<()>(state)).layer(with_principal(...)))`.
   - The router's own `admin_gate` ([router.rs#L48](../../../crates/starter-authz/src/routes/router.rs#L48)) already requires `Role::Admin`, so no extra `with_role` wrapper is needed.

5. **Audit sink** — wire `DbDecisionSink` into the policy engine's decision path so `GET /v1/authz/decisions` returns rows. Without this the panel renders an empty list even after enforcement is live.

### 0.4 Risk callouts

- **Lockout risk.** Step 2 flips the tools-router gate's behavior. If the bootstrap admin rule (step 2 final bullet) is not seeded atomically with the schema, the first deploy of this change denies every tool call. Treat the seed as part of the migration, not a follow-up.
- **Static engine consumers.** Anything else in the agent that calls `boot::authz::build_engine()` (see [`services.rs#L78`](../../crates/rubix-agent/src/bin/rubix_agent/services.rs#L78), [`boot/mcp/register.rs#L67`](../../crates/rubix-agent/src/boot/mcp/register.rs#L67)) needs to handle the new concrete type or a shared trait object — audit before the swap.
- **`DELETE /v1/tenants/{id}` is intentionally absent** ([tenants.rs#L20](../../../crates/starter-auth-users/src/routes/tenants.rs#L20)). The UI's "Delete tenant" affordance in Section 2.4 needs to be either soft-delete via PATCH or removed from the wireframe.

### 0.5 Suggested staging

To avoid a flag-day:

- **Stage A** — mount `tenants_router` only. Tenants/Members/Teams panels start working end-to-end; the rest of the authz UI keeps 404'ing (hide those tabs behind a `RUBIX_AUTHZ_ENABLED` flag, or render an "Unavailable" empty state). No security posture change.
- **Stage B** — swap to `DbPolicyEngine` + seed bootstrap admin rule + mount `authz_router` + wire `DbDecisionSink`. Ship behind the same flag so it can be rolled back without a redeploy.
- **Stage C** — the UI redesign in Sections 2–6.

Stages A and B are the prerequisite. Stage C is what the rest of this document covers.

---

## 1. Current state

### 1.1 Routes

- [`rubix/frontend/src/routes/admin/access.tsx`](../../frontend/src/routes/admin/access.tsx#L172) renders `<AuthzAdmin>` from `@nube/starter-ui-authz` inside a page chrome.
- [`rubix/frontend/src/routes/admin/users.tsx`](../../frontend/src/routes/admin/users.tsx#L42) is an entirely separate page that uses `useUserList / useUserCreate / useUserDisable / useUndoLast` from `@nube/rubix-client-react`. It does not talk to authz at all.

So today the operator has two unrelated admin pages whose data sets (rubix users + authz subjects) overlap conceptually but are reached via different navigation items and rendered by different code paths.

### 1.2 Tabs in `<AuthzAdmin>`

Tab order is defined at [`packages/starter-ui-authz/src/panels/authz-admin.tsx`](../../../packages/starter-ui-authz/src/panels/authz-admin.tsx#L76-L86): Tenants, Members, Teams, Rules, Assignments, Resources, Check, Decisions. Selected-tenant state lives on the shell ([authz-admin.tsx#L65](../../../packages/starter-ui-authz/src/panels/authz-admin.tsx#L65)) and is passed to Members + Teams only.

Concrete pain points, panel by panel:

- **Tenants** ([tenants-panel.tsx](../../../packages/starter-ui-authz/src/panels/tenants-panel.tsx#L60)) — create form + table. Rows are clickable to "select" the tenant, but the selection state is invisible once you leave the tab. There is no breadcrumb or chip telling the operator "you are scoped to acme" while looking at Rules or Assignments.
- **Members** ([members-panel.tsx](../../../packages/starter-ui-authz/src/panels/members-panel.tsx#L77)) — empty-state "Select a tenant to manage its members" with no link or picker to actually select one. The add form takes a free-text `User id` ([members-panel.tsx#L103](../../../packages/starter-ui-authz/src/panels/members-panel.tsx#L103-L110)) — the operator must know the opaque id. The panel comment ([members-panel.tsx#L1-L11](../../../packages/starter-ui-authz/src/panels/members-panel.tsx#L1-L11)) explicitly admits "the server has no `GET /v1/tenants/{id}/members` listing endpoint", so the list area is empty in the uncontrolled mode the rubix host uses.
- **Teams** ([teams-panel.tsx](../../../packages/starter-ui-authz/src/panels/teams-panel.tsx#L62)) — same tenant-prompt pattern. Each row has a tiny inline "add member by user id" input ([teams-panel.tsx#L117-L138](../../../packages/starter-ui-authz/src/panels/teams-panel.tsx#L117-L138)) — again free-text, no autocomplete, no listing of who is already in the team.
- **Rules** ([rules-panel.tsx](../../../packages/starter-ui-authz/src/panels/rules-panel.tsx#L80)) — a single grid form with 7 inputs (role, resource, actions, effect, condition, priority, tenant). Tenant picker here is *independent* of the shell-level tenant selection ([rules-panel.tsx#L54](../../../packages/starter-ui-authz/src/panels/rules-panel.tsx#L54)) — they should be linked.
- **Assignments** ([assignments-panel.tsx](../../../packages/starter-ui-authz/src/panels/assignments-panel.tsx#L29)) — the worst offender. Subject is a free-text input whose placeholder is literally `user-id or user-*` ([assignments-panel.tsx#L66](../../../packages/starter-ui-authz/src/panels/assignments-panel.tsx#L66)). Operators must hand-type subject ids to bind roles. The form lives in a tab divorced from any user/team context.
- **Resources** — read-only catalog. Fine where it is but unrelated to the daily workflow.
- **Check** ([check-panel.tsx]) — also asks for `Principal subject` as a free-text input (see screenshot `/tmp/ac-screens/access-check.png`). Same pain.
- **Decisions** — paged audit log. Filters take a free-text `Subject` for the same reason.

### 1.3 `/admin/users` ([users.tsx#L42](../../frontend/src/routes/admin/users.tsx#L42))

- Lists rubix-level users (email, role, status), supports create + disable + undo. No tenant concept at all.
- The "role" here is a rubix system role (e.g. `operator`), separate from the tenant-scoped roles (`reader|writer|admin`) defined at [`members-panel.tsx#L38`](../../../packages/starter-ui-authz/src/panels/members-panel.tsx#L38).
- "Member" vs "User" is undefined for an operator. Today: a User is a rubix account; a Member is a (user, tenant, role) triple. The UI does not explain this.

### 1.4 Summary of pain points

1. No hierarchy. Tenant -> team -> user is a hard mental model but the UI presents 8 sibling tabs.
2. Users and Members are two different concepts on two different pages with no cross-linking.
3. Every "subject" or "user id" input is free-text. There is no picker, no validation, no preview.
4. The shell-level `tenantId` state is set on the Tenants tab but invisible everywhere else.
5. Rules, Assignments, Check, Decisions ignore the selected tenant.
6. The proxy/data error visible on `/tmp/ac-screens/access-tenants.png` ("Expected JSON from /v1/tenants...") suggests this surface is rarely exercised end-to-end; nobody is using it because it is too painful.

---

## 2. Proposed information architecture

Single `/admin/access` page. `/admin/users` is removed. The page is a master-detail layout: a left rail with the tenant -> team -> user tree (the "directory"), and a right pane that shows scoped detail tabs for the selected entity. Global tools (Resources, Decisions, Check) move to a top-right toolbar of the page itself.

### 2.1 Primary nav (left rail)

```
Tenants
├── acme                       [click -> tenant detail]
│   ├── Teams
│   │   ├── platform           [click -> team detail]
│   │   └── facilities
│   └── Members                [click -> tenant members list]
│       ├── alice@acme.io
│       └── bob@acme.io
└── globex
    └── ...
```

- The rail is the only place tenant/team/user selection happens.
- Selecting any node sets URL state, e.g. `/admin/access/t/acme/team/platform`, so deep links work and breadcrumb derives from URL.
- "Members" under a tenant is the merged former Users + Members concept (see Section 5 Open questions).

### 2.2 Detail pane (right) — scoped to selected entity

| Selected node | Tabs in detail pane |
| --- | --- |
| (nothing) | Overview, Decisions (global), Resources (global), Check (global) |
| Tenant | Overview, Teams, Members, Rules (filtered to this tenant + global), Assignments, Decisions (filtered) |
| Team | Overview, Members (of the team), Rules (filtered), Assignments (for this team) |
| User | Profile, Memberships (which tenants/teams/roles), Assignments, Decisions (for this subject) |

Key wins:
- A "user" finally has a single page that answers "what can they do".
- Rules / Assignments / Decisions inherit the selected scope; the operator doesn't re-pick the tenant on every tab.
- Resources and Check stay global but live in a toolbar, not in the main tabstrip.

### 2.3 Wireframe — landing

```
+-----------------------------------------------------------------------------+
| Admin / Access Control                  [Resources] [Check] [Decisions]     |
+-----------------------------------------------------------------------------+
| Tenants  + new       |  Overview                                            |
| > acme               |  -------------------------------------------------   |
|   > Teams            |   3 tenants, 24 members, 7 teams, 41 rules           |
|     - platform       |                                                      |
|     - facilities     |   Recent activity                                    |
|   > Members (12)     |   - assignment created: alice -> writer@acme         |
| > globex             |   - rule added: reader can read flow:*               |
| > nube-dev           |   - member added: bob to acme/platform               |
|                      |                                                      |
| [search...]          |                                                      |
+-----------------------------------------------------------------------------+
```

### 2.4 Wireframe — tenant detail

```
+-----------------------------------------------------------------------------+
| acme  >  Tenant                                  Edit  •  Delete            |
+-----------------------------------------------------------------------------+
| [Overview] [Teams] [Members] [Rules] [Assignments] [Decisions]              |
+-----------------------------------------------------------------------------+
| Members in acme                              + Add member                   |
| -------------------------------------------------------------------------   |
|  Email                       Team(s)           Tenant role     Actions      |
|  alice@acme.io               platform, fac.    writer          [Edit] [x]   |
|  bob@acme.io                 facilities        reader          [Edit] [x]   |
|  carl@acme.io                -                 admin           [Edit] [x]   |
+-----------------------------------------------------------------------------+
```

The "+ Add member" button opens a dialog with a searchable user picker (see Section 3) and a role select. No more typing of opaque ids.

### 2.5 Wireframe — user detail

```
+-----------------------------------------------------------------------------+
| alice@acme.io  >  User                       Disable  •  Reset password     |
+-----------------------------------------------------------------------------+
| [Profile] [Memberships] [Assignments] [Decisions]                           |
+-----------------------------------------------------------------------------+
| Memberships                                  + Add to tenant/team           |
| -------------------------------------------------------------------------   |
|  Tenant     Team        Role        Added by      When                      |
|  acme       platform    writer      op@nube       2026-05-12                |
|  acme       facilities  reader      op@nube       2026-05-13                |
|  globex     -           admin       op@nube       2026-04-01                |
+-----------------------------------------------------------------------------+
| Assignments (direct subject bindings)        + Add assignment               |
| -------------------------------------------------------------------------   |
|  Subject              Role         Created by      When                     |
|  alice@acme.io        flow-runner  op@nube         2026-05-20               |
+-----------------------------------------------------------------------------+
```

---

## 3. Assignment UX — kill the free-text input

The single biggest user complaint. Today: [`assignments-panel.tsx#L60-L73`](../../../packages/starter-ui-authz/src/panels/assignments-panel.tsx#L60-L73) is a raw `<Input>` over `subject`. Proposal:

1. Replace the `<Input id="a-subj">` with a `<UserPicker>` component (new). It takes a search query and a `mode: 'user' | 'team' | 'glob'`.
2. Default mode is `user`. Search is debounced and calls `useUserList()` (the same hook `/admin/users` uses today, [users.tsx#L36](../../frontend/src/routes/admin/users.tsx#L36)). Results render as `email — role — userId` rows.
3. A `mode: 'team'` segment lets the operator pick a team instead. The picker emits a synthetic subject string `team:<id>` (backend support TBD, see Section 5 #2). Until backend supports it, hide the team mode behind a feature flag.
4. A `mode: 'glob'` segment keeps the power-user free-text path for `user-*` style globs but is hidden behind an "Advanced" disclosure.
5. The "Create assignment" action moves out of the standalone Assignments tab and lives on:
   - the user detail pane ("Assignments" tab), with the subject prefilled to the current user;
   - the team detail pane ("Assignments" tab), with subject prefilled to `team:<id>`;
   - the global Decisions/Check toolbar action (advanced).
6. The standalone "Assignments" *list* survives as a tenant-scoped tab so an operator can audit "all direct bindings under acme" without drilling into individual users.

Same treatment for:
- Members "User id" input ([members-panel.tsx#L103](../../../packages/starter-ui-authz/src/panels/members-panel.tsx#L103-L110)) -> `<UserPicker mode="user">`.
- Teams team-member draft input ([teams-panel.tsx#L117-L130](../../../packages/starter-ui-authz/src/panels/teams-panel.tsx#L117-L130)) -> `<UserPicker>`.
- Check `Principal subject` and `Resource owner` -> `<UserPicker>` with a "use my subject" shortcut.
- Decisions filter `Subject` -> `<UserPicker>` (single-select, optional).

### 3.1 `UserPicker` data source

`useUserList()` already exists at [`rubix-client-react`](../../frontend/src/routes/admin/users.tsx#L36). It returns `{ users: [{ user_id, email, role, disabled_at_ms }] }`. The picker can use that immediately for the user mode. For team mode, we already have `useTeams(tenantId)` at [`hooks/index.ts`](../../../packages/starter-ui-authz/src/hooks/index.ts#L43).

---

## 4. Migration plan

### 4.1 Delete

- [`rubix/frontend/src/routes/admin/users.tsx`](../../frontend/src/routes/admin/users.tsx) — replace with a redirect to `/admin/access` (or to `/admin/access?focus=members`).
- Any sidebar/nav link to `/admin/users` (search the layout components). The single nav item becomes "Access Control".

### 4.2 Refactor in `@nube/starter-ui-authz`

- [`panels/authz-admin.tsx`](../../../packages/starter-ui-authz/src/panels/authz-admin.tsx) — strip the 8-tab `<Tabs>`; replace with a `<MasterDetail>` shell that owns `selectedNode: {kind: 'tenant'|'team'|'user', id}` state and renders a left rail + scoped right pane. Tenant selection moves from the Tenants panel into the rail.
- [`panels/tenants-panel.tsx`](../../../packages/starter-ui-authz/src/panels/tenants-panel.tsx) — becomes the empty-state overview + create-tenant CTA; the table moves into the left rail as the tenant list.
- [`panels/members-panel.tsx`](../../../packages/starter-ui-authz/src/panels/members-panel.tsx) — already controlled-friendly. Rubix host passes a real list via `useUserList()` + filter by tenant membership. Empty-state prompt at [members-panel.tsx#L84](../../../packages/starter-ui-authz/src/panels/members-panel.tsx#L84) is no longer needed because the panel only renders when a tenant is selected in the rail.
- [`panels/teams-panel.tsx`](../../../packages/starter-ui-authz/src/panels/teams-panel.tsx) — drop the row-level inline member input; team detail becomes its own pane reached by clicking a team in the rail.
- [`panels/assignments-panel.tsx`](../../../packages/starter-ui-authz/src/panels/assignments-panel.tsx) — split into `<AssignmentsList scope={...}>` and `<AssignmentCreateDialog defaultSubject={...}>`. The standalone "Assignments" tab keeps only the list (tenant-scoped). Creation moves to user/team detail panes.
- [`panels/rules-panel.tsx`](../../../packages/starter-ui-authz/src/panels/rules-panel.tsx) — accept `tenantId` prop and prefill the tenant select ([rules-panel.tsx#L156-L165](../../../packages/starter-ui-authz/src/panels/rules-panel.tsx#L156-L165)) from the master-detail context.
- [`panels/check-panel.tsx`] and [`panels/decisions-panel.tsx`] — accept optional `subject` and `tenantId` from context; render `<UserPicker>` instead of free-text input.

### 4.3 Add

- `packages/starter-ui-authz/src/panels/user-picker.tsx` — new component. Search-as-you-type combobox over the rubix users API + an optional team mode. Returns `{ kind, id, label }`.
- `packages/starter-ui-authz/src/panels/master-detail.tsx` — new layout shell. URL-state-aware so deep links to `/admin/access/t/acme/u/alice` work.
- A small `useUserDirectory()` adapter hook on the rubix side so `@nube/starter-ui-authz` does not have to depend on `@nube/rubix-client-react` directly. Pattern: host passes a `userDirectory: { search, getById }` prop into `<AuthzAdmin>`. Mirrors how Members already accepts a `members` prop ([members-panel.tsx#L42-L48](../../../packages/starter-ui-authz/src/panels/members-panel.tsx#L42-L48)).
- A new route file [`rubix/frontend/src/routes/admin/access.$.tsx`](../../frontend/src/routes/admin/access.$.tsx) (catch-all) so the master-detail can read `t/<slug>/team/<slug>` etc. from the URL. The existing [`access.tsx`](../../frontend/src/routes/admin/access.tsx#L200) becomes a thin wrapper that mounts `<AuthzAdmin userDirectory={...}>`.

### 4.4 Suggested (optional) backend additions

These would make the redesign noticeably better but are not blocking:

- `GET /v1/tenants/{id}/members` — today the comment at [members-panel.tsx#L1-L11](../../../packages/starter-ui-authz/src/panels/members-panel.tsx#L1-L11) says this endpoint does not exist. Without it the rubix host must reconstruct membership from `useUserList()` + per-user lookups, which is fine but lossy.
- `subject: "team:<id>"` support in assignments, so binding a role to a team becomes first-class.
- A `GET /v1/users?q=` search endpoint (or extend `useUserList()` to take a query) — the current hook loads everything client-side.

### 4.5 Phasing

1. **Phase 1 — picker only.** Ship `<UserPicker>` and wire it into Members, Teams team-member input, and Assignments subject input. Keeps the existing tab IA. Single-PR-sized. Highest ROI fix for the headline complaint.
2. **Phase 2 — merge Users into Access.** Move `/admin/users` content into a "Users" tab on `/admin/access`. Redirect old route. Still flat tabs.
3. **Phase 3 — master-detail.** Replace flat tabs with the tenant-tree rail + scoped detail panes. New URL scheme. Largest change; do behind a feature flag for one release.

---

## 5. Open questions

1. **"Members" vs "Users" terminology.** Today a User is a rubix account and a Member is `(user, tenant, role)`. Do we keep both terms (with explainer text) or rebrand the rail node as "People"? Recommendation: keep "Users" for the rubix-account list and "Members" inside a tenant pane.
2. **Team as a subject.** Should assignments accept `subject: "team:<id>"`? If yes, `<UserPicker>` exposes the team mode immediately; if no, hide it. This is a backend product decision.
3. **Glob subjects (`user-*`).** Power-user feature in [assignments-panel.tsx#L66](../../../packages/starter-ui-authz/src/panels/assignments-panel.tsx#L66). Keep them but hide behind an "Advanced" toggle in the picker. Should globs also be allowed on members? (They currently can't be — Members goes through `addTenantMember` which expects a real user.)
4. **Multi-tenant URL scoping.** Proposed scheme `/admin/access/t/<slug>/...` uses tenant *slug*. The existing API uses *id* ([authz-admin.tsx#L65](../../../packages/starter-ui-authz/src/panels/authz-admin.tsx#L65)). Decision: prefer slug in URLs for readability, resolve to id internally via `useTenants()` cache.
5. **Rubix system role vs tenant role.** `/admin/users` sets a rubix role (`operator`); tenants set a tenant role (`reader|writer|admin`). Where does the rubix role live in the new IA? Recommendation: on the User detail pane's "Profile" tab.
6. **Resources tab placement.** Strictly a read-only reference. Top-right toolbar or a "Catalog" tab on the root view? Recommendation: toolbar drawer.
7. **Decisions volume.** Decisions can be huge. When scoped to a single subject/tenant the panel is useful; ungated it is noise. Should the global toolbar Decisions view default to "last 24h" with a required filter?
8. **Undo last.** [`useUndoLast`](../../frontend/src/routes/admin/users.tsx#L38) is wired to `/admin/users` only. Should it follow the merge and apply to authz mutations too?

---

## 6. Concrete first PR

Smallest change that addresses the loudest complaint:

- Add `packages/starter-ui-authz/src/panels/user-picker.tsx` (combobox over `useUserList`-style adapter).
- Add a `userDirectory` prop to `<AuthzAdmin>` ([authz-admin.tsx#L36](../../../packages/starter-ui-authz/src/panels/authz-admin.tsx#L36)) and thread it down.
- Replace the three free-text subject/user-id inputs:
  - [assignments-panel.tsx#L60-L73](../../../packages/starter-ui-authz/src/panels/assignments-panel.tsx#L60-L73)
  - [members-panel.tsx#L101-L111](../../../packages/starter-ui-authz/src/panels/members-panel.tsx#L101-L111)
  - [teams-panel.tsx#L117-L130](../../../packages/starter-ui-authz/src/panels/teams-panel.tsx#L117-L130)
- In rubix host [access.tsx](../../frontend/src/routes/admin/access.tsx#L187) pass `userDirectory={{ search: useUserList... }}`.

No URL changes, no nav changes, no backend changes. Ships the picker. Phases 2 and 3 follow.
