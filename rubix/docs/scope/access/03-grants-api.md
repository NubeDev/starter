# G3 — Grants API + page-drawer mutations

> Scope tier. See [README](./README.md). Backend + frontend stage.

## Goal

Make the read-only drawer from G2 editable. An operator picks a
team or user, picks View / Edit / Manage, and the system stores
it as a rule the existing engine already understands. No new
authz concept; this is sugar on top of `RuleStore`.

## The mapping (no engine change)

A grant `{subject: "team:hvac-ops", action: "edit", resource:
{kind: "rubix.dashboard.page", id: "dash_01HX..."}, effect: "allow"}`
is persisted as a single rule row:

```jsonc
{
  "role": "team:hvac-ops",
  "resource": "rubix.dashboard.page",
  "resource_id": "dash_01HX...",
  "actions": ["edit", "view"],   // tier expansion (Edit implies View)
  "effect": "allow",
  "condition": null,             // no condition — straight ACL
  "priority": 100,               // grants outrank seed defaults but lose to deny
  "tenant_id": "<current>",
  "source": "grant"              // new marker column, see migration below
}
```

Tier → actions expansion for `rubix.dashboard.page`:

| Tier | Actions written |
|---|---|
| View | `["view"]` |
| Edit | `["view", "edit"]` |
| Manage | `["view", "edit", "delete"]` |

For other kinds with `instances_provider` later, the tier map is
declared on the kind registration. v1 only ships
`rubix.dashboard.page`.

### Why `source: "grant"`?

So G2's `has_legacy_rules` flag stays accurate: any rule with
`source = "grant"` is round-trippable; everything else is
"legacy" and the UI tells the user to edit it in Advanced mode.
Without this marker we can't tell a hand-written rule that
happens to look like a grant apart from one written by the
grants API.

### Role-resolution change

The policy engine already does substring match on `role`. The
`team:<slug>` role string is **synthesised at principal-build
time** from the user's team memberships: when a request comes in
for user `u` in tenant `t`, we look up `u`'s teams and the
principal's `roles` set gains `team:<slug>` for each. This is a
one-line addition in the principal builder, not an engine
change.

## Backend

### Migration

Single additive migration on `starter_authz_rules`:

```sql
ALTER TABLE starter_authz_rules
  ADD COLUMN source TEXT NOT NULL DEFAULT 'manual',
  ADD COLUMN resource_id TEXT;
CREATE INDEX idx_rules_resource_instance
  ON starter_authz_rules (resource, resource_id, tenant_id)
  WHERE source = 'grant';
```

- Existing rows default to `source = 'manual'` (correct — they
  were hand-written or seeded).
- `resource_id` was previously baked into the `resource` field
  for some seeds; the migration leaves those alone. The
  evaluator already handles `resource_id IS NULL` as
  "kind-level rule"; we just stop overloading the resource
  string for new grants.

### New routes

```
POST   /v1/authz/grants
  body: { subject, resource_kind, resource_id, tier, effect }
  → 201 { id, ...grant }

DELETE /v1/authz/grants/:id
  → 204

GET    /v1/authz/grants
  ?subject=<team:slug | user:sub>
  &resource_kind=<kind>
  &resource_id=<id>
  → [{ id, subject, resource_kind, resource_id, tier, effect, source }]

PATCH  /v1/authz/grants/:id
  body: { tier? }    // only tier is mutable; subject/resource is delete+create
  → 200 { ...grant }
```

All gated by admin role on the target tenant. `GET` is what G4
(team Permissions view) consumes.

### Share-scope helpers

The three radios in the drawer also map to grants:

| Radio | Effect on rules table |
|---|---|
| Private | Delete every `source='grant'` rule on this resource_id. Owner check still allows the owner (engine ownership condition). |
| Tenant | Delete grants, write one rule `{ role: "*", resource, resource_id, actions: ["view"], source: "grant" }`. |
| Specific | No-op on its own; the rows in the list below are the actual grants. |

These are implemented as **server-side helpers** so the UI
sends one `PUT /v1/authz/grants/share-scope/:resource_kind/:resource_id`
call with `{ scope: "private" | "tenant" | "specific" }` and the
backend reconciles. The UI doesn't manually delete-and-recreate.

## Frontend

The G2 drawer becomes writable:

- `+ Add team or person` opens a combobox listing this tenant's
  teams + members; pick one, pick a tier, fires `POST /grants`,
  drawer refetches.
- The tier dropdown on each row fires `PATCH /grants/:id`.
- `✕` on each row fires `DELETE /grants/:id`.
- The share-scope radios fire the share-scope helper.
- `Legacy rules` badge stays read-only and links to
  Advanced → Rules pre-filtered.

## Files touched

Backend:
- New migration file under
  [`crates/starter-authz/migrations/`](../../../../crates/starter-authz/migrations/).
- [`crates/starter-authz/src/grants.rs`](../../../../crates/starter-authz/src/grants.rs)
  *(new)* — `GrantStore` thin wrapper over `RuleStore` that
  enforces the tier-expansion + source marker.
- [`crates/starter-authz/src/routes.rs`](../../../../crates/starter-authz/src/routes.rs)
  — add the four `/grants` routes + share-scope helper.
- [`crates/starter-auth-users/src/principal.rs`](../../../../crates/starter-auth-users/src/principal.rs)
  — synthesise `team:<slug>` role strings from memberships.

Frontend:
- `panels/page-detail-drawer.tsx` — wire mutations.
- `panels/grants-combobox.tsx` *(new)* — subject picker.
- `client/grants.ts` *(new)* — typed client for the four routes.

## Tests

Backend:
- `grants::create::expands_edit_tier_to_view_plus_edit`
- `grants::create::writes_source_marker`
- `grants::delete::removes_only_the_target_row`
- `grants::patch::tier_update_rewrites_actions_in_place`
- `grants::share_scope::tenant_writes_wildcard_subject_view_rule`
- `grants::share_scope::private_deletes_all_grant_rows_keeps_owner_path`
- `principal::team_roles::membership_yields_team_slug_role`
- End-to-end: create a grant, hit `POST /v1/authz/check` for the
  team member on the page → Allow; for a non-member → Deny.

Frontend:
- `page-detail-drawer.test.tsx` — add/remove/patch fire the
  right requests; optimistic update + refetch on success.
- `grants-combobox.test.tsx` — lists teams first, then members.

## Verification

```bash
cargo test -p starter-authz -p starter-auth-users
cargo clippy -p starter-authz -p starter-auth-users -- -D warnings
pnpm -F starter-ui-authz test

# smoke: grant team edit on a page, check it resolves
curl -s -X POST http://127.0.0.1:8088/v1/authz/grants \
  -H "$AUTH" -H 'content-type: application/json' \
  -d '{"subject":"team:hvac-ops","resource_kind":"rubix.dashboard.page","resource_id":"<seeded-id>","tier":"Edit","effect":"allow"}'

curl -s -X POST http://127.0.0.1:8088/v1/authz/check \
  -H "$AUTH" -H 'content-type: application/json' \
  -d '{"principal":"<hvac member>","action":"edit","resource":{"kind":"rubix.dashboard.page","id":"<seeded-id>"}}' \
  | jq '.decision'
```

Playwright smoke:
1. Open a page drawer, click `+ Add team or person`, pick HVAC
   Ops, pick Edit, confirm.
2. Assert row appears, refresh, assert it persists.
3. Switch tier to View, assert dropdown updates and request fired.
4. Click `✕`, assert row removed.
5. Switch share scope to Tenant, assert wildcard chip appears
   and a `*` row is shown read-only in the list.

## Out of this stage

- Per-team Permissions view (G4 consumes the GET endpoint
  shipped here).
- Tier vocabulary for non-page kinds (deferred).
- Rule-conflict warnings ("an explicit Deny on the same resource
  will override this grant") — captured as follow-up.
