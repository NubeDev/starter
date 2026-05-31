# Access — scope (Admin UX redesign for tenants / teams / resources)

> **Tier:** scope (plan). Lifetime: weeks. Per
> [HOW-TO-CODE.md §0a](../../../HOW-TO-CODE.md) **source code must
> not reference any file in this folder.** When a stage lands,
> promote the present-tense parts into `docs/design/authz/` and
> point code links there.

## What this scope is

The plan to make Access Control (`/admin/access`) **usable by a
human operator who is not a security engineer**. Today's UI
exposes the policy engine's primitives (Rules, Assignments,
Decisions, Resources) directly; operators bounce off it because
the mental model required to grant "HVAC Ops can edit the Boiler
Overview page" is "write a rule with a condition referencing the
team slug." Grafana, Notion, Linear all converge on a different
model: **permissions live on the thing you're protecting**, you
pick the thing, then you pick who.

This scope keeps the engine and its schema intact. It is an
**admin-UX redesign + a thin resource-instance API** on top of
[`starter-authz`](../../../../crates/starter-authz/) and
[`starter-auth-users`](../../../../crates/starter-auth-users/).

This scope does NOT introduce a new authz concept. Rules,
assignments and decisions stay — they move behind an
**Advanced mode** toggle for power users and debugging.

## What we already have (no design needed)

Verified by inspection on the workspace as of this scope draft:

| Substrate | Status | Source |
|---|---|---|
| Tenants, teams (slug + display name), memberships with role (reader/writer/admin) | ✅ shipping | [`crates/starter-auth-users/`](../../../../crates/starter-auth-users/) |
| Policy engine — rules (role, resource, action, effect, condition, priority), assignments (subject→roles), decisions (Allow/Deny + reason) | ✅ shipping | [`crates/starter-authz/`](../../../../crates/starter-authz/) |
| Resource registry — registrable kinds with actions, ownership model, `tenant_scoped` flag | ✅ shipping | [`crates/starter-authz/src/registry.rs`](../../../../crates/starter-authz/src/registry.rs) |
| Registered kinds today: `rubix.tool` (invoke), `rubix.dashboard.page` (view/edit/delete, owner=Subject, tenant-scoped) | ✅ shipping | rubix host boot |
| HTTP surface — `/v1/authz/rules`, `/assignments`, `/resources`, `/check`, `/decisions` | ✅ shipping | [`crates/starter-authz/src/routes.rs`](../../../../crates/starter-authz/src/routes.rs) |
| Frontend admin panel with Overview/Teams/Members/Rules/Assignments/Decisions tabs | ✅ shipping | [`packages/starter-ui-authz/src/panels/authz-admin.tsx`](../../../../packages/starter-ui-authz/src/panels/authz-admin.tsx) |
| Dashboard pages persisted in `dashboards_definitions` with creator + tenant | ✅ shipping | [`crates/starter-dashboards/`](../../../../crates/starter-dashboards/) |

## What's broken about today's UX

Captured from the current screenshot at `/admin/access/t/system`
and confirmed by code survey:

1. **The flow reads backwards.** Operators land on Access Control
   and see stats, then Rules and Decisions — engine concepts —
   before they ever see a team or a page. The intuitive flow is
   `Tenant → Team → grant access to a thing`; the UI surfaces
   `Rule → Assignment → Decision`.
2. **No resource-centric view.** There is no "list of pages, click
   to toggle team access" — the Grafana / Notion pattern. All
   permission edits go through the raw rule editor.
3. **Teams cannot be granted permissions directly.** Today, to
   give a team edit-access to a page, an operator must hand-write
   a rule with a condition expression like
   `principal.teams contains "hvac-ops"`. There is no UI sugar.
4. **No resource-instance enumeration.** `GET /v1/authz/resources`
   lists kinds (catalogue), not instances. There is no
   `GET /v1/authz/pages` to drive a list-of-pages view.
5. **Power-user noise on every screen.** Rules, Assignments,
   Decisions are useful for debugging once a quarter; they sit on
   the top-level tab bar all day, every day.

## What we have to build

Four substrate additions. None of them change the engine's
evaluation semantics — they expose existing data in shapes the UI
needs, and add a sugar layer that round-trips to rules.

| Gap | What it is | Stage |
|---|---|---|
| **G1. Simple/Advanced mode + tab restructure** | Frontend-only. Default Simple mode shows `Teams / Members / Pages`. Advanced toggle unlocks `Rules / Assignments / Audit log`. State persisted per-user. | [01-simple-mode-and-ia.md](./01-simple-mode-and-ia.md) |
| **G2. Resource-instance API + Pages list** | Backend `GET /v1/authz/resources/:kind/instances` returns instances scoped to current tenant with effective permissions. Frontend Pages tab consumes it. | [02-resource-instances.md](./02-resource-instances.md) |
| **G3. Team-as-subject grants (`grant` API + sugar)** | Backend `POST /v1/authz/grants` that takes `{subject: team_slug | user, resource: kind+id, action, effect}` and writes the equivalent rule row. Frontend "Add team or person" picker calls this. Round-trips to the same rules table; no schema change. | [03-grants-api.md](./03-grants-api.md) |
| **G4. Per-team Permissions tab** | Frontend team detail view with a Permissions sub-tab that lists every grant where this team is the subject. Backend: `GET /v1/authz/grants?subject=team:hvac-ops`. | [04-team-permissions-view.md](./04-team-permissions-view.md) |

A fifth document covers everything we deliberately keep out of
v1:

| Out of scope | [05-out-of-scope.md](./05-out-of-scope.md) |
|---|---|

## Mental model the redesigned UI enforces

```
Tenant
├── Teams           who
│   ├── HVAC Ops    (members + permissions)
│   └── Energy
├── Members         people in this tenant + their tenant-role
└── Resources       what
    └── Pages       (SDUI / dashboard pages, gateable per team or user)
```

**Simple mode** shows exactly this. Three top-level tabs:
`Teams`, `Members`, `Pages`. The team detail view has a
`Permissions` sub-tab. The page detail drawer has a "Who can
access" list. Nothing else.

**Advanced mode** layers back the engine view:
`Rules`, `Assignments`, `Audit log` (renamed from Decisions, same
data). The Resources catalogue moves to a dev-tools drawer.

## Decisions taken in this scope (no longer open)

These were the open questions in the proposal; resolved here so
the implementation stages don't re-open them:

1. **Role vocabulary in Simple mode.** Fixed three-tier:
   **View / Edit / Manage**. They map to existing actions per
   kind (for `rubix.dashboard.page`: `view`, `edit`, `delete+edit`).
   Advanced mode still exposes raw actions for resources with
   non-standard verbs. Rationale: Grafana and Notion both ship
   3-tier in their default UI and only expose verbs in the
   power-user sheet; mirroring that lets us reuse the mental
   model operators already have.
2. **Share scopes in the page drawer.** Three radios:
   `Private (owner only) / Tenant (any member) / Specific teams or people`.
   Tenant is kept because operators repeatedly want
   "everyone in Acme can view this dashboard" without enumerating
   teams.
3. **Extensions as gateable resources.** Out of v1. Pages only.
   Captured in [05-out-of-scope.md](./05-out-of-scope.md).
4. **Tools tab.** Out of v1 Simple mode. The kind stays
   registered; Advanced mode rules can still gate it.

## Implementation plan — autonomous session

The four stages above are sized to land in **one autonomous
Claude Code session** per the
[autonomous session prompt](../../../AUTONOMOUS-SESSION-PROMPT.md).

### Session shape

- **Driver (main thread):** reads this README and the five stage
  docs end-to-end, then lands stages **G1 → G2 → G3 → G4** in
  order. Each stage commits independently with tests green.
- **Parallel research sub-agents (Stage 0, before any edits):**
  spawn in a single message with multiple Agent calls so they run
  concurrently:
  1. **Explore — backend authz surface.** Map every callsite of
     `RuleStore`, `AssignmentStore`, `decision_audit`. Confirm
     existing route handlers we'll extend in G2/G3 don't have
     hidden invariants.
  2. **Explore — frontend authz panel.** Enumerate every
     component imported by `panels/authz-admin.tsx` so the
     IA restructure in G1 doesn't break a co-located sub-view.
  3. **Explore — dashboard page persistence.** Confirm
     `dashboards_definitions` schema (id, tenant, creator,
     updated_at) and existing list endpoint shape, since G2
     proxies it for the Pages tab.
  4. **Explore — current rule shapes in seed data.** Find every
     seeded rule that gates `rubix.dashboard.page` or `rubix.tool`
     today; G3's `grants` API must produce rows that read back
     correctly through the existing `/rules` UI.
- **Plan sub-agent (after research returns):** consume the four
  Explore reports and produce a step-ordered file-by-file plan
  for G1–G4. Driver edits from that plan.
- **Verification sub-agents (one per stage):** after each stage
  commit, spawn a Verification agent with a fixed script:
  `cargo test -p starter-authz`,
  `cargo clippy -p starter-authz -- -D warnings`,
  `pnpm -F starter-ui-authz test`,
  plus a `curl` smoke against the new/changed endpoint, plus a
  Playwright headed smoke for the Simple-mode flow at
  `/admin/access`. Verification agent returns pass/fail + logs;
  driver only proceeds to next stage on pass.

### Stop conditions for the autonomous run

Standard rules from `AUTONOMOUS-SESSION-PROMPT.md` §9 apply.
Project-specific additions:

- **Do not migrate or rename existing rule rows.** The grants
  sugar in G3 writes new rows in the canonical rule shape;
  pre-existing rules are left untouched. If a Pages list view
  in G2 surfaces a page that has *only* condition-based
  legacy rules (no team-subject grant), it's rendered as
  read-only with an "Edit in Advanced mode" link — not silently
  rewritten.
- **Do not change the resource registry signature.** G2 adds an
  `instances_provider` hook on the existing `ResourceKind`
  registration; the trait gains a default-empty method, no kind
  is forced to implement it.
- **Schema changes.** G3 ships the additive migration on
  `starter_authz_rules` (`source TEXT NOT NULL DEFAULT 'manual'`
  + `resource_id TEXT` + the partial index) as written in
  [03-grants-api.md](./03-grants-api.md). The DB is not in
  production; if the migration breaks an existing dev DB, dropping
  and reseeding is acceptable. Any *additional* migration the
  implementer discovers mid-stage still requires stopping and
  writing it explicitly.
- **Principal `team:<slug>` synthesis is unconditional.** G3
  injects `team:<slug>` into every principal's roles set from
  team memberships, no feature flag. The change is additive
  (legacy condition-based rules still evaluate identically), and
  any incidental new matches via role are intentional — grants
  are Allow-only and Deny still wins.

### Per-stage commit shape

Each stage's commit message:

```
access(GX): <one-line shipped behavior>

- backend: <files touched>
- frontend: <files touched>
- tests: <new test names>
- verification: <curl + browser smoke result>
```

### Verification baseline (capture before starting)

```
# tabs visible today
curl -s http://127.0.0.1:8088/openapi.json | jq '.paths | keys | map(select(startswith("/v1/authz")))'

# what a rule looks like today (so G3's sugar writes the same shape)
curl -s http://127.0.0.1:8088/v1/authz/rules -H "$AUTH" | jq '.[0]'

# pages exist?
curl -s http://127.0.0.1:8088/v1/dashboards/pages -H "$AUTH" | jq 'length'
```

## What promotes to `docs/design/authz/` on land

When G1–G4 ship, the present-tense parts of:
- the mental model section above,
- the Simple/Advanced tab structure,
- the three-tier role vocabulary,
- the grants API shape,

move into `docs/design/authz/admin-ux.md` and this scope folder
shrinks to a one-paragraph "delivered" pointer per existing scope
convention.
