# G2 — Resource-instance API + Pages tab

> Scope tier. See [README](./README.md). Backend + frontend stage.

## Goal

Surface a list of actual dashboard pages in the admin UI, with
their effective access (who can view / edit / manage), so
operators can click a page and toggle teams or users without
ever opening the raw rule editor.

This is the Grafana folder-permissions view, transposed onto our
`rubix.dashboard.page` resource kind.

## Why an instances API at all

`GET /v1/authz/resources` lists kinds (catalogue) — it has no
idea what pages exist; pages live in `dashboards_definitions`,
not in authz tables. We need a way for the authz UI to ask
"give me the instances of kind X in this tenant, with their
current ACL summary" without the UI having to know which crate
owns each kind.

## Backend

### New trait method on the registry

`ResourceKind` registration grows an optional `instances_provider`:

```rust
pub trait InstancesProvider: Send + Sync {
    async fn list(
        &self,
        principal: &Principal,
        tenant_id: TenantId,
        query: InstancesQuery,
    ) -> Result<InstancesPage>;
}

pub struct InstancesQuery {
    pub search: Option<String>,
    pub cursor: Option<String>,
    pub limit: u32,
}

pub struct InstancesPage {
    pub items: Vec<ResourceInstance>,
    pub next_cursor: Option<String>,
}

pub struct ResourceInstance {
    pub id: String,
    pub label: String,
    pub owner: Option<SubjectRef>,
    pub effective_acl: EffectiveAcl,
}

pub struct EffectiveAcl {
    pub share_scope: ShareScope,           // Private | Tenant | Specific
    pub grants: Vec<GrantSummary>,         // already-resolved subject→tier
    pub has_legacy_rules: bool,            // true if condition-based rules exist
}

pub struct GrantSummary {
    pub subject: SubjectRef,               // team:<slug> or user:<sub>
    pub tier: PermissionTier,              // View | Edit | Manage
}
```

Default impl returns "not supported"; only kinds that opt in get
listed by the new endpoint.

### Registering the provider for `rubix.dashboard.page`

`rubix-host` boot registers an instances provider that wraps
`starter-dashboards::PageStore::list_by_tenant`. To avoid N+1 on
tenants with many pages, the provider:

1. Loads the page slice (cursor + limit) once.
2. Loads **all rules** where
   `resource.kind == "rubix.dashboard.page"` and tenant matches
   in a single query (matching rows are bounded by tenant — a few
   hundred at the top end).
3. Groups rules in memory by `resource_id` (with `NULL`/`"*"`
   bucketed as the tenant-wide fallback applied to every page).
4. For each page, derives the `EffectiveAcl` from its bucket:
   - Bucket by subject (team or user); pick highest tier per
     subject (Manage > Edit > View).
   - `share_scope`: `Private` if zero non-owner grants; `Tenant`
     if a wildcard-subject `view` rule exists; `Specific`
     otherwise.
   - `has_legacy_rules`: true if any matching rule has a
     non-empty `condition` field. These are the hand-written
     rules that predate the grants sugar (G3).

The summariser logic lives in `crates/starter-authz/src/acl.rs`
and is the same code path G4 calls for per-team views.

### New route

```
GET /v1/authz/resources/:kind/instances
  ?search=<text>
  &cursor=<opaque>
  &limit=<n, default 50, max 200>
  → InstancesPage
```

- Gated by the existing admin-role check; no new permission.
- Returns 404 if the kind has no `instances_provider` registered.
- Tenant scope is implicit (principal's current tenant); admins
  with global scope pass `?tenant=<slug>`.

## Frontend

### Pages tab

`/admin/access/t/:tenantSlug/pages`. Table columns:

| Page | Access | Owner | Updated |
|---|---|---|---|

- **Access** column renders compact pills derived from
  `EffectiveAcl`:
  - `🔒 Private` if `Private`.
  - `🌐 Tenant (view)` if `Tenant`.
  - `🟢 HVAC Ops (edit) +2 more` if `Specific` — first chip plus
    a `+N more` if more than one grant.
  - `⚠ Legacy rules` badge if `has_legacy_rules` is true (clicks
    deep-link to the Rules tab in Advanced mode, filtered by
    this page id).
- Search input filters server-side (`?search=`).
- Cursor pagination at the bottom.

Click a row → page detail drawer (rendered, edit lands in G3):

```
┌──────────────── Boiler Overview ────────────────┐
│ Owner: alice@acme.com                            │
│ Page ID: dash_01HX...                            │
│                                                  │
│ Share with:                                      │
│   ( ) Private                                    │
│   ( ) Anyone in this tenant                      │
│   (•) Specific teams or people                   │
│                                                  │
│ WHO CAN ACCESS                                   │
│ ┌──────────────────────────────────────────┐    │
│ │ 🟢 HVAC Ops      [Edit  ▼]    ✕          │    │
│ │ 👁 Viewers       [View  ▼]    ✕          │    │
│ │ 👤 bob@acme.com  [Edit  ▼]    ✕          │    │
│ └──────────────────────────────────────────┘    │
│                                                  │
│ [+ Add team or person]  ← wired in G3            │
└──────────────────────────────────────────────────┘
```

In G2 the drawer is **read-only**: the `+ Add`, `✕`, and the
share-scope radios all open a tooltip *"Editing access lands in
the next stage"* and the underlying mutation hooks are stubbed.
G3 fills them in.

## Files touched

Backend:
- [`crates/starter-authz/src/registry.rs`](../../../../crates/starter-authz/src/registry.rs)
  — add `InstancesProvider` trait + opt-in registration.
- [`crates/starter-authz/src/routes.rs`](../../../../crates/starter-authz/src/routes.rs)
  — add `GET /resources/:kind/instances`.
- [`crates/starter-authz/src/acl.rs`](../../../../crates/starter-authz/src/acl.rs)
  *(new)* — `EffectiveAcl` summariser shared by G2 and the
  team-permissions view in G4.
- [`rubix/crates/rubix-host/src/authz.rs`](../../../../rubix/crates/rubix-host/src/authz.rs)
  — wire a `DashboardPageInstances` provider that delegates to
  `starter-dashboards`.

Frontend:
- [`packages/starter-ui-authz/src/panels/pages-tab.tsx`](../../../../packages/starter-ui-authz/src/panels/pages-tab.tsx)
  *(new)*.
- [`packages/starter-ui-authz/src/panels/page-detail-drawer.tsx`](../../../../packages/starter-ui-authz/src/panels/page-detail-drawer.tsx)
  *(new)* — read-only in this stage.
- `panels/authz-admin.tsx` — mount Pages tab in Simple mode.

## Tests

Backend (`starter-authz`):
- `routes::instances::lists_pages_for_tenant`
- `routes::instances::unknown_kind_returns_404`
- `routes::instances::respects_search_and_cursor`
- `acl::summariser::buckets_grants_picks_highest_tier`
- `acl::summariser::flags_legacy_rules_with_conditions`
- `acl::summariser::detects_tenant_share_scope_from_wildcard_subject`

Frontend:
- `pages-tab.test.tsx` — renders pills from fixture, search box
  fires server query, legacy badge appears for fixture with
  `has_legacy_rules: true`.
- `page-detail-drawer.test.tsx` — read-only state in G2 (buttons
  show tooltip, no mutation fires).

## Verification

```bash
cargo test -p starter-authz
cargo clippy -p starter-authz -- -D warnings
pnpm -F starter-ui-authz test

# smoke
curl -s "http://127.0.0.1:8088/v1/authz/resources/rubix.dashboard.page/instances?limit=5" \
  -H "$AUTH" | jq '.items[0]'
```

Playwright smoke:
1. Log in, navigate `/admin/access/t/system/pages`.
2. Assert table renders at least one row from the seeded
   dashboards.
3. Click a row, assert drawer opens with owner and share-scope
   radios in read-only state.
4. Type into search, assert table refetches.

## Out of this stage

- Any mutation from the drawer (G3).
- Tools / Extensions instance lists (deferred; the
  `instances_provider` hook makes them mechanical to add later).
