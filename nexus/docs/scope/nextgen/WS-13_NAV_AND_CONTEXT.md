# WS-13 — Navigation Tree, Page Context & Access (reuse one page across a fleet)

> **Status:** Backend + context + nav UI landed on `nexus-gaps` (migration/store, DTO-first nav API + C6 audit/undo, tag-authz fix, `context` VariableKind full-stack, PageContext assembly + query-key wiring, nav-based access provider/seed, nav tree sidebar + builder + Access Navigation tab). Follow-ups (file-ownership blocked this run): remove the old dashboard-toolbar Share path + unused `DashboardAccessTab`; mount `TagEditor kind="dashboard"`; auto-seed tenant-view grants on tenant *create* (no framework hook today — seeded via `seed-admin`). · **Wave:** 3 · **Owner:** `nexus-gaps`
> **Depends on:** WS-02 variables (context resolves *into* the variable layer), WS-05 C1 JSON model + folders (a nav node points at a dashboard), tags backend (`0005_tags.sql` + `routes/tags/**`, already shipped), the generic authz seam (`authz/mod.rs` registry + `InstancesProvider`) · **owns** the nav-based access model (replaces per-dashboard sharing) · pairs with WS-02
> **Coordinate with authz/WS-12:** this WS **deletes** the per-`nexus.dashboard` Access surface and makes `nexus.nav_node` the sole navigation-grant kind — confirm no other WS depends on the old per-page share path before removing it.
> **Migration:** block `17xx` (`1701_nav_tree.sql`; tags reuse `nexus_tags` — no new tag table) · **Read first:** WS-02, WS-05, GAP_ANALYSIS §2.5
> **Verified:** `nexus-gaps` on 2026-06-10 — peer-reviewed and re-grepped: tag write is `PUT /api/v1/tags/{kind}/{id}` full-replace + **tenant-only, no per-resource authz/existence check** (`routes/tags/set.rs:27`, `nexus-store/src/tag/mod.rs`); `VariableKind` is a **closed enum** (`nexus-spi/.../variable/shared.rs:13`, `ui/src/data/types.ts:264`); `nexus_dashboards.id` is a **global PK** with `tenant_id` a separate column (`0002_dashboards.sql:6`) so a bare FK does *not* encode same-tenant; panel query key is `[…, sql, tick, interval, varRevision]` with **no context term** (`ui/src/features/widgets/useWidgetQuery.ts:60`); a reusable `TagEditor` exists, mounted for datasources only (`ui/src/features/tags/TagEditor.tsx`, `features/datasources/DatasourceRow.tsx:83`).

## Goal
Make **one dashboard page serve a whole fleet** by separating the *page* (a parameterised
template) from the *place* it's viewed for (a building, a level, a site). Today a page is reused only
by hand-picking variables in the bar. WS-13 adds two coupled pieces:

1. **Page context** — a page can read the **URL**, its **nav-tree position**, and its **tags**, and
   feed those into variables/queries. So `dashboard-page-abc` rendered under *Building-1* queries
   Building-1's data; the same page under *Building-2* queries Building-2's — no second page authored.
2. **A navigation tree** — a nested, user-built nav (a new sidebar tab) where each node **assigns a
   page and a context payload**. `buildings → building-1 (page-abc, {building: b1}) → level-1`. Click
   a node → open its page bound to its context.

3. **Nav-based access** — because the nav node (not the page) is what a user navigates, **access is
   granted per node**, for *both* dashboard pages and static app pages (Agents, Datasources, Audit,
   …). The Access section's per-dashboard sharing is **replaced** by per-node sharing. Granting
   *Building-1* to a user gives them that mount — not every building that reuses the same template.

Together: author one page, mount it at many nav nodes, each node parameterises *and* gates it. This
is the fleet-scaling story WS-02 started, finished at the navigation + access layer.

## Current state (evidence)
- **Flat sidebar list only** (`ui/src/features/dashboards/SidebarDashboards.tsx`); routes by slug
  (`ui/src/app/router.tsx:26` — `d/:slug`). No nesting, no nav nodes, no per-node context.
- **Folders exist but are organisational, not navigational** (`0602_folders.sql`, WS-05): a folder
  groups dashboards in a tree, but a folder node does **not** carry a context payload and a dashboard
  appears under exactly one folder. A nav node is different — it *binds a page + context* and the
  **same page** may hang off many nodes. Keep them distinct (see Design notes).
- **Variables resolve, but only from their own config** (`ui/src/features/variables/resolve.ts`):
  `constant | custom | query | datasource | interval | textbox` + built-ins. There is **no source
  that reads URL / nav-node / tags** — that's the gap this WS fills. Built-ins (`$__dashboard`,
  `$__user`, `$__from`, `$__to`) are already carved out as a `$__`-prefixed namespace
  (`ui/src/features/variables/deps.ts:20-21`) — context built-ins extend that namespace.
- **Tags are built backend-side but unwired on dashboards — and the write path is *tenant-only*.**
  `nexus_tags` is a generic `(entity_type, entity_id, key, value)` store (`0005_tags.sql`) swept on
  dashboard delete (`nexus-store/src/dashboard/delete.rs`). The API is **full-replace
  `PUT /api/v1/tags/{kind}/{id}`** (`routes/tags/mod.rs:27`, *not* `POST /tags`), read `GET` on the
  same path, reverse-lookup `GET /api/v1/tags/entities/{kind}`, and **keys-only autocomplete**
  `GET /api/v1/tags/keys` (`keys.rs:13` — there is **no values endpoint**). **Critical gap:** the
  write handler resolves only the **tenant** (`set.rs:33`) — it does **not** check `edit` on the
  target dashboard, and because `nexus_tags` is polymorphic with no FK, it will tag a **nonexistent**
  id. A reusable `TagEditor` already exists, mounted for **datasources only**
  (`ui/src/features/tags/TagEditor.tsx`, `features/datasources/DatasourceRow.tsx:83`). So "tags on a
  dashboard page" reuses that component but **must first close the authz/existence gap** below — it is
  not pure wiring.

## Scope

### 1. Page context model (C1 extension — coordinate with WS-05)
A page resolves against a **`PageContext`** assembled at view time from four *named sources*. The
sources are kept **separate** (not pre-flattened) so a `context` variable can address exactly one
(`source: 'nav' | 'url' | 'tag' | 'values'`) and the precedence is explicit and testable:
```
PageContext {
  nav?:   { nodeId, slug, name, path: string[] },   // the nav node the page was opened under
  url:    Record<string, string | string[]>,        // query params (var-* and bare)
  tags:   Record<string, string | null>,            // this dashboard's tags (key → value|null)
  values: Record<string, string | string[]>,        // NavNode.context.values — explicit overrides
}
```
**Where `NavNode.context` lands (the merge contract — implementers must not invent this):** a nav
node's payload is **not** spread as arbitrary top-level keys. It is exactly:
`NavNode.context = { values?: Record<string,string|string[]>, tags?: Record<string,string|null> }`.
- `context.values` → `PageContext.values` (a node setting `{ values: { building: "b1" } }` makes
  `$building` resolvable via a `context`/`values` variable). This is the path the `building: "b1"`
  example uses — it is a **values override**, not a magic top-level key.
- `context.tags` → merged **over** the dashboard's own tags into `PageContext.tags` (a node can
  pin/override a tag for its mount without retagging the shared page).
- There is **no `varOverrides`** key — it was ambiguous with `values`; dropped. A node overrides a
  variable's *current value* through `values` + a `context` variable, which then flows the normal
  WS-02 selection path. (If a future need arises for "force this variable regardless of bar", spec it
  then; do not leave two overlapping override channels.)

**Resolution precedence (later wins), applied per variable name:** dashboard tags → `nav.values`
(i.e. `context.values`) → URL params → explicit variable-bar selection. The context is **read-only
input to variable resolution**, not a fourth persistence store. It is assembled in
`ui/src/features/variables/context.ts` (new) and threaded into `resolveOptions` / selection seeding.

### 2. New variable kind: `context` (extends WS-02 — this is a *persisted kind*, full stack)
**Decision: `context` is a new first-class `VariableKind`, not a sub-source.** `VariableKind` is a
**closed enum in three places** (`nexus-spi/.../variable/shared.rs:13`, the store, and
`ui/src/data/types.ts:264`); adding a value is **not** "just `resolve.ts` work." Adding it means, as a
checklist (all in WS-13's acceptance):
1. `VariableKind::Context` in the Rust DTO (`nexus-spi/.../variable/shared.rs`) + its kind-config
   struct; `openapi.rs` → `openapi.json` → **`pnpm codegen`** to regenerate the TS client.
2. `"context"` in the TS `VariableKind` union (`ui/src/data/types.ts:264`) + `parseKindConfig`
   (`features/variables/config.ts`) for the new config shape.
3. A `resolve.ts` arm (synchronous, no fetch) + the variable **form** (`VariableForm.tsx`) authoring
   UI + the editor dialog.
4. **Export/import** (WS-05): a `context` variable serialises into the dashboard JSON like any other.
5. Tests at each layer (DTO round-trip, config parse, resolve-per-source, form).

Config shape: `{ source: 'nav' | 'url' | 'tag' | 'values', key }` →
- `nav` + `key=slug|name|path[n]` → the nav node's slug/name/ancestor.
- `url` + `key=building` → `?building=…` (WS-02 owns `?var-*`; `context`/`url` reads a **bare**
  `?building=b1` so external deep-links drive the page without knowing variable internals).
- `tag` + `key=building` → the dashboard's `building` tag value (from `PageContext.tags`).
- `values` + `key=building` → `PageContext.values[building]` (i.e. the nav node's `context.values`).

- A `query` variable then references a `context` variable as a parent (cascading already works):
  `WHERE building = '$building'`. **No new interpolation** — it rides the WS-03 binder like any other
  variable (injection-safe by construction; a `context` value binds as a `$N` arg, never inlined).
- **Built-in context tokens** (no authoring, always present): `$__nav_slug`, `$__nav_name`,
  `$__tag(key)`. Register in the `$__` namespace alongside `$__dashboard`/`$__user`. These are
  resolver-side tokens, **not** enum values, so they need no DTO change.

### 3. Dashboard tags UI + **close the tag authz gap** (security work, not just wiring)
- **Backend (must land first):** the tag write path is **tenant-only** (`set.rs:33`) — it neither
  checks `edit` on the target resource nor that the `{kind}/{id}` entity exists. Tags are now
  **behaviour-affecting inputs** (they drive queries via `PageContext.tags`), so an arbitrary
  same-tenant caller tagging any/nonexistent dashboard is a real authz hole. **Required:** before
  `set_for_entity`/`get`, resolve the entity per `kind` and enforce the **same authz the resource's
  own routes use** — for `dashboard`, reuse `authz/dashboard_instances.rs` (`edit` to write, `view`
  to read); reject unknown/foreign ids with 404/403. Do this generically in the tag handlers (a
  per-kind "resolve + authorize" step), so it also covers `datasource`. This is a fix to the shared
  tag routes — coordinate so other consumers (datasource TagEditor) get it too.
- **Frontend:** reuse the existing `TagEditor` (`ui/src/features/tags/TagEditor.tsx`) — mount it on
  the dashboard with `kind="dashboard"`. The write is **full-replace `PUT /api/v1/tags/{kind}/{id}`**
  (`routes/tags/mod.rs:27`), not a `POST` append. Read is `GET` on the same path. Key autocomplete is
  `GET /api/v1/tags/keys` (**keys only** — there is no values endpoint today; either add a
  `GET /api/v1/tags/values?key=` route here or scope the UI to key autocomplete only and state which).
- Tags become an input to `PageContext.tags` and to the `tag` context source above.
- **No tag migration** — `nexus_tags` already exists, is RLS-scoped and delete-swept; the change is
  handler authz, not schema.

### 4. Navigation tree — **the single navigation + access surface** (new sidebar tab)
New backend resource + new sidebar UI. **Decision (committed):** the nav tree is the **one place
access is granted** for everything a user navigates to — dashboard pages *and* static app pages
(Agents, Datasources, Alerts, Flows, Explore, Audit, Access). There is **no per-page grant path**
kept alongside it; the old per-`nexus.dashboard` Access tab is **replaced** (not in production, zero
backward-compat — see §6). A nav node is the unit of "can this user see this". A nav tree is a
tenant-scoped, nestable set of nodes:
```
NavNode {
  id, tenant_id,
  parent_id?,                 // self-ref, NULL = root; nestable arbitrarily deep
  title,                      // display label ("Buildings", "Building-1", "Agents")
  sort_order,
  // --- what the node points at: exactly one of (dashboard | route | none) ---
  target: jsonb,              // tagged union:
                              //   { kind: "group" }                       → non-clickable header
                              //   { kind: "dashboard", dashboardId }       → a reusable page mount
                              //   { kind: "route", route: "agents" }       → a static app page
  context?: jsonb,            // dashboard targets only. EXACTLY { values?: …, tags?: … } — §1 merge.
  icon?, accent?              // optional, match dashboard appearance (0006_dashboard_appearance.sql)
}
```
The `route` kind is a **closed allow-list** of the app's built-in pages (the router table,
`ui/src/app/router.tsx`) — `agents | datasources | alerts | flows | explore | audit | access |
dashboards`. It is *not* free-form text (a node can't point at an arbitrary URL). This is what lets
a static page like **Agents** be access-gated by a node, exactly like a dashboard page.

- `1701_nav_tree.sql` (WS-13 `17xx` block): `nexus_nav_nodes` table, RLS + FORCE RLS + tenant policy
  + `nexus_runtime` grants + a `(tenant_id, parent_id, sort_order)` index — **mirror the
  `0602_folders.sql` shape exactly**. `target` is JSONB (the tagged union above); a `dashboard`
  target's id is validated in the handler (next bullet) rather than by a DB FK, because the union
  shape can't carry a typed column FK and (below) the FK wouldn't be tenant-safe anyway. On dashboard
  delete, sweep dependent nodes to `{ kind: "group" }` in the dashboard delete path (same place tags
  are swept, `nexus-store/src/dashboard/delete.rs`) — losing a page must not delete the nav node.
- **Same-tenant invariant for a `dashboard` target (do not rely on a bare FK):** `nexus_dashboards.id`
  is a **global PK** with `tenant_id` a separate column (`0002_dashboards.sql:6`), so a bare
  `REFERENCES` would let a node point at **another tenant's** dashboard. **Require:** the
  create/update handler validates the referenced dashboard exists **within the caller's tenant**
  (tenant-scoped `SELECT` under RLS) and that the caller has `edit` on the nav tree. (Note: viewing
  the *node* is now the grant that matters — see §6 — so we no longer require the author to hold
  `view` on the underlying page; the node grant supersedes it.)
- **CRUD + reorder/reparent endpoints** under `routes/nav/**`; DTO in `nexus-spi/src/dto/nav/**` →
  `openapi.rs` → `openapi.json` → `pnpm codegen` (the house DTO-first flow).
- **Sidebar UI** (`ui/src/features/nav/**`): replaces `SidebarDashboards.tsx` as the primary sidebar.
  A nested, collapsible tree; `group` nodes expand/collapse; `dashboard`/`route` nodes are links.
  **The signed-in user sees only the nodes they hold `view` on** (the tree is filtered server-side by
  the grant check — §6). An **editor** (admin) to add/nest/reorder nodes, pick a target
  (dashboard + context, or a static route), and manage each node's grants inline.
- **Routing**: a `dashboard` node opens `d/:slug?nav=:nodeId` → page reads `nav` → loads node →
  merges `node.context` into `PageContext` (§1). A `route` node navigates to that static page. Same
  page components; only the entry point and (for dashboards) the context differ. Query-param nav id
  keeps the page route stable and shareable.

### 5. Resolution wiring + **context must be in the query keys** (concrete, not hand-wavy)
On dashboard load: assemble `PageContext` → seed `context` variables and bare URL params → resolve
the rest (cascading) in the existing WS-02 order. The re-query path today is **revision-based**, so
context must feed those exact keys — naming them so an implementer can't miss one:
- **Panel query key** is `["nexus","query",datasourceId,sql,tick,interval,varRevision]`
  (`ui/src/features/widgets/useWidgetQuery.ts:60`). It has **no context term**. The clean fix is to
  fold resolved context into the **variable values** that drive `varRevision` — i.e. context resolves
  *into* the variable selections, so a context change bumps `varRevision` like any selection change.
  If any context can reach a panel **without** going through a variable, it must be added to this key
  explicitly; prefer routing everything through variables so there is **one** revision to bump.
- **Variable-resolution key** is `[...variablesKey(slug), JSON.stringify(selections)]`
  (`ui/src/features/variables/useDashboardVariables.ts:117`) — keyed on `slug + selections` only.
  Since `slug` is unchanged when only the nav node changes, the assembled context (nav/tag/url) **must
  be part of what `selections` contains** (or be added to this key) or option lists for cascading
  `query` variables will resolve stale when navigating between two nodes of the same page.
- Fold the same resolved context into the **WS-09 server cache key (C3)** so two nav mounts of one
  page don't collide on a shared cache entry.

Changing the nav node (navigating) re-assembles context → updates selections → bumps `varRevision` →
re-queries exactly the dependent panels. **Reuse WS-02's dependency-driven invalidation; do not add a
parallel one** — the whole point is that context arrives *as variable values*.

### 6. Access model — **grant on the nav node, for every page** (replaces per-dashboard sharing)
This WS **moves the access surface from the page to the nav node** and **deletes the old per-page
grant path** (no production, zero backward-compat per the user's instruction). The reasoning: once a
page is reused across many nav mounts, "who can see this page" is the wrong question — a user should
get *Building-1*, not "the energy-overview template wherever it appears". The node is what a user
navigates, so the node is what's granted.

**Mechanism (reuse the existing generic authz seam — do not invent a parallel one):**
- Register a new resource kind **`nexus.nav_node`** in `authz/mod.rs` (`register_nexus_resources`,
  standard `view|edit|delete` actions, `tenant_scoped`). This is the *only* kind the Access UI grants
  for navigation.
- Add a **`NavNodeInstancesProvider`** mirroring `dashboard_instances.rs` (the `InstancesProvider`
  seam `starter-authz` already drives): it lists the tenant's nav nodes + each node's effective ACL
  so the Access UI renders share-scope (private / tenant / specific) + grants per node — identical
  UX to today's `DashboardAccessTab`, just over nodes.
- **Enforcement on navigation:** the nav-tree `GET` filters to nodes the principal holds `view` on
  (`authz::can(engine, principal, "view", KIND_NAV_NODE, nodeId, tenant)` — the existing `can`
  filter-helper, `authz/mod.rs:81`). The sidebar therefore *is* the access-filtered tree: a user only
  sees Building-1 if granted on Building-1's node.
- **Enforcement on the page itself:** opening `d/:slug?nav=:id` checks `view` on the **node**, not the
  dashboard. A `route` node (e.g. Agents) checks `view` on the node and, if allowed, the static page
  renders. **The page/dashboard/datasource kinds keep their `edit`/`delete` grants for *authoring***
  (who may modify the template) — but **`view`-to-navigate is now a node grant**. So:
  - `nexus.nav_node` `view` → can navigate to it (the new, primary check).
  - `nexus.dashboard` / `nexus.datasource` / … `edit`/`delete` → can author/manage the underlying
    asset (unchanged; still listed under their own admin surfaces, not the navigation tab).
- **Static pages get gated for the first time.** Today routes like `agents`/`audit`/`access` are
  reachable by any tenant member (router-only, no grant). Under this model a `route` node carries the
  grant, so an admin can hide *Audit* or *Access* from non-admins by not granting their nodes. Seed a
  **default tree** on tenant create (every static route as a node, granted `tenant` scope) so nothing
  silently disappears — the gating is opt-in tightening, not a lockout.

**What is deleted (no legacy):** the `DashboardAccessTab` "manage sharing per dashboard" surface and
its `nexus.dashboard`-instance Access listing. The dashboard toolbar **Share** button either retargets
to "grant the node(s) that mount this page" or is removed in favour of the nav-node grant editor —
decide in the editor UX (§7). Do **not** keep both grant paths alive.

### 7. UX workflow (the end-to-end flow this WS delivers)
**A. Admin builds the navigation** (new *Navigation* builder, `ui/src/features/nav/**`):
1. Open **Navigation** (new top-level admin area / sidebar-edit mode).
2. **Add a group node** — `+ Group` → title "Buildings". Non-clickable header; just organises.
3. **Add a page mount under it** — `+ Page` → title "Building-1" → pick target **Dashboard** →
   choose the reusable page `energy-overview` → author **context**: `values: { building: "b1" }`
   (and optionally pin a `tag`). Repeat: "Building-2" → same `energy-overview` page → `building: "b2"`.
   *One page, two mounts, two buildings.*
4. **Add a static page** — `+ Page` → title "Agents" → pick target **Route** → choose `agents` from
   the allow-list. (Same for Datasources, Alerts, Audit, …)
5. **Nest / reorder** by drag — arbitrary depth (`buildings → building-1 → level-1`).
6. **Grant access per node, inline** — each node has a **Manage access** affordance (the same
   permissions drawer the Access section uses): set scope `private | tenant | specific people/teams`.
   Granting "Building-1" to *alice* lets alice see that node and open its page; she does **not**
   automatically get "Building-2" even though it's the same template.

**B. End user navigates** (the everyday flow):
1. Signs in → sidebar shows **only the nodes they hold `view` on** (server-filtered tree).
2. Expands "Buildings" → clicks "Building-1" → opens `energy-overview` bound to `building=b1`;
   panels query Building-1's data. Clicks "Building-2" → same page, `building=b2`, re-queried.
3. Clicks "Agents" (a `route` node they're granted) → the static Agents page renders.
4. A deep link `d/energy-overview?nav=<building-1-node>` restores the exact same context-bound view
   (and is access-checked on that node).

**C. Access administration** (the *Access* section, restructured — see §6):
1. Open **Access** → the **Navigation** tab (replaces the old *Dashboards* tab) lists the nav tree;
   each node shows its share scope + grant count + **Manage** (the existing permissions drawer over
   `nexus.nav_node`).
2. **Teams** / **Members** tabs are unchanged.
3. Authoring grants (who may *edit* a dashboard template or a datasource) live under those assets'
   own admin surfaces, not here — the Access section is now strictly "who can navigate where".

## Design notes
- **Nav tree ≠ folders.** Folders *file* a dashboard once for organisation; a nav node *mounts* a
  (possibly shared) page with a context. The same `dashboard-page-abc` legitimately appears under
  Building-1 and Building-2 nav nodes — that's the whole point and folders can't express it. Build a
  separate `nexus_nav_nodes` table; do **not** overload `nexus_folders`.
- **Context is precedence-merged, never persisted as a 4th store.** Tags persist in `nexus_tags`,
  nav context in `nexus_nav_nodes.context`, URL in the URL — `PageContext` is the *resolved view* of
  those three at render time. This keeps export/import self-contained (WS-05): a page's behaviour is
  its variables; the *place* (nav/url/tags) is external input, exactly like a Grafana template var.
- **Injection boundary is unchanged.** Context values become `QueryVariable`s and bind through the
  WS-03 binder as `$N` args — a nav node named `'); DROP …` is quoted, never executed. Re-use the
  WS-02 safety story verbatim; this WS adds *sources*, not a new interpolation path.
- **Bare URL params vs `var-*`.** WS-02 owns `?var-region=…` (explicit variable state). WS-13's `url`
  context source additionally reads **bare** params (`?building=b1`) so a deep link / external system
  can drive a page without knowing variable-bar internals. Document both; don't collide the prefixes.
- **No backwards migration** (not in production): the nav table and tag-wiring ship forward-only; no
  data backfill of the existing flat sidebar — it stays as the default tree's root listing.

## Acceptance criteria
- [ ] **C6 (audit/undo):** `nav_node` kind has a `Reversible` impl + `record_if_reversible` in its
  create/update/delete/reorder handlers + is in WS-12's mutable-kinds manifest; create/move/delete a
  nav node produces a `Change` row and is undoable. Tag edits record under the **dashboard** kind
  (tags are dashboard sub-state) — confirm with WS-12, don't double-record.
- [ ] **`context` is a full-stack `VariableKind`:** added to the Rust DTO enum + config struct,
  regenerated through `openapi.json`/`pnpm codegen`, added to the TS union + `parseKindConfig` +
  `resolve.ts` + `VariableForm`, and round-trips through WS-05 export/import. A `context` variable
  resolves from nav / url / tag / values; a `query` panel re-queries when the nav node changes;
  cascading from a `context` parent works.
- [ ] **Tag authz gap closed:** the tag write/read handlers resolve the `{kind}/{id}` entity and
  enforce the resource's own authz (`edit`/`view` via `authz/dashboard_instances.rs` for dashboards)
  and reject unknown/foreign ids — a same-tenant caller can **not** tag a dashboard they can't edit
  or a nonexistent id. Dashboard tag editor (reused `TagEditor`, `kind="dashboard"`) writes via
  full-replace `PUT /api/v1/tags/dashboard/{id}`; `$__tag(building)` and a `tag`-source variable
  resolve to it.
- [ ] **Same-tenant nav→dashboard invariant enforced:** a nav node cannot reference another tenant's
  dashboard (handler validates tenant-scoped existence); test proves a cross-tenant `dashboard`
  target is rejected.
- [ ] `NavNode.context` merges per the §1 contract (`values` → `PageContext.values`, `tags` over
  dashboard tags); there is **no** `varOverrides` channel.
- [ ] Nav tree: create/nest/reorder/reparent; `group` / `dashboard` / `route` targets; a `dashboard`
  node opens its page with context applied; a `route` node opens its static page; deleting a page
  sweeps its nodes to `{ kind: "group" }` (no nav rows lost).
- [ ] **Nav-based access (replaces per-page sharing):** `nexus.nav_node` is registered with a
  `NavNodeInstancesProvider`; the sidebar/nav `GET` is server-filtered to nodes the principal holds
  `view` on; opening a node checks `view` on the **node**, not the page. A user granted "Building-1"
  but not "Building-2" sees and opens only Building-1 though both reuse one template. The old
  per-`nexus.dashboard` Access tab + `DashboardAccessTab` are **removed** (no dual grant path).
- [ ] **Static pages gated:** a `route` node (e.g. Agents, Audit) is hidden from a user not granted
  its node; a fresh tenant gets a **default tree** of all static routes (granted `tenant`) so nothing
  silently disappears.
- [ ] **C6 for nav grants:** granting/revoking a node is audited + undoable like other grant changes
  (confirm with WS-12 whether grant rows already flow through the changelog).
- [ ] **Context is in the query keys:** changing the nav node bumps `varRevision`
  (`useWidgetQuery.ts:60`) and re-keys variable resolution (`useDashboardVariables.ts:117`) and the
  WS-09 cache key — two nav mounts of one page never serve each other's cached panels.
- [ ] One page authored once, mounted at two nav nodes with different `context.values`, renders two
  different buildings' data — **no second page**.
- [ ] Context deep-links: opening `d/:slug?nav=:id` (and bare `?building=…`) restores the same view.
- [ ] Injection test: a nav title / tag value containing SQL metacharacters is safely bound, never
  executed.
- [ ] Tests: `PageContext` precedence merge, `context` resolution per source, nav CRUD + reorder +
  RLS isolation + cross-tenant rejection, tag authz (edit/view enforced, unknown id rejected),
  context→`varRevision`/cache-key inclusion, dependency-driven re-query on nav change.

## Out of scope (hand off)
- The variable model, interpolation, and cascading engine → **WS-02 / WS-03** (this WS only adds
  `context` as a *source* and the `$__` context tokens; it does not touch the binder).
- The dashboard JSON model + folders + export/import shape → **WS-05** (coordinate the `PageContext`
  read-path and the `d/:slug` routing; WS-13 adds the `nav` query param, not a new page route).
- Repeat-by-variable rendering → **WS-05** (a nav tree is navigation, not in-canvas repeat; they
  compose — a repeated panel under a context-bound page — but neither owns the other).
