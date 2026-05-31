# Access G1–G4 — Implementation Plan (autonomous session output)

> Output of the Stage-0 Plan sub-agent (see README.md §"Implementation
> plan — autonomous session"). Driver follows this file-by-file.
> Where this plan diverges from a stage doc, the divergence is called
> out inline and wins.

Conventions:
- "RuleStore" in scope docs = `PolicyStore` trait
  (`crates/starter-authz/src/store/mod.rs`).
- "registry.rs" in scope docs = the registry seam is split:
  `crates/starter-spi/src/authz/registry.rs` (trait + spec, do not
  change signature) and `crates/starter-authz/src/registry.rs`
  (impl). The `InstancesProvider` hook lands in **starter-authz**
  (next to the impl), not the SPI.
- "starter-dashboards" = `rubix/crates/rubix-store-postgres/src/dashboards/`
  exposed via the `DashboardStore` trait at
  `rubix/crates/rubix-spi/src/dashboard/store.rs`.
- Scope doc says `rubix-host/src/authz.rs` — actual file is
  `rubix/crates/rubix-agent/src/boot/authz.rs`.
- **Principal team-slug synthesis**: rather than adding a
  `Principal.roles: Vec<String>` field and mutating both verify
  functions, synthesise `team:<slug>` strings inside
  `DbPolicyEngine::roles_for` (engine.rs:108–120) by iterating
  `p.teams`. One-line, additive, no SPI change, zero risk to
  non-authz callers.
- **Tests**: per user resolution, no new vitest setup for
  starter-ui-authz. Frontend verification = `tsc --noEmit`
  (typecheck) + Playwright smoke. Backend = `cargo test -p starter-authz`.

---

## G1 — Simple/Advanced mode + IA restructure (frontend only) — DONE

Shipped:
- `packages/starter-ui-authz/src/panels/mode-toggle.tsx` (new)
- `packages/starter-ui-authz/src/panels/index.ts` (re-export)
- `packages/starter-ui-authz/src/panels/authz-admin.tsx` — wired
  toggle into the header, threaded `mode` prop through DetailPane,
  gated TenantDetail and TeamDetail advanced tabs (Rules /
  Assignments / Audit log) behind `mode === "advanced"`,
  renamed the Decisions tab label to "Audit log", hid the
  Resources / Check / Decisions drawer buttons in Simple mode.

Deferred to follow-up (not load-bearing for G2–G4):
- URL-segment-driven tabs (`/admin/access/t/:slug/pages` etc.) — needs
  `splatToSelected` to grow a tab dimension. Today tabs are local
  state. Pages tab in G2 still mounts under the TenantDetail's
  TabsContent; deep-link comes later.
- One-time redirect toast on bookmarked Advanced-only URL in Simple
  mode.

Verification: `pnpm -F starter-ui-authz typecheck` ✅

---

## G2 — Resource-instance API + Pages tab (backend + frontend)

### Files to create
- `crates/starter-authz/src/acl.rs` — pure summariser. Inputs:
  `tenant_id`, `Vec<StoredRule>` pre-filtered to one kind, and a
  page iterator (`id`, `owner`, `label`). Outputs per page:
  `EffectiveAcl { share_scope, grants, has_legacy_rules }`.
  Bucket by `resource_id` (treat `None` and `"*"` as tenant-wide);
  pick highest tier per subject (Manage > Edit > View);
  `has_legacy_rules = any rule.condition.is_some()`. Tier inference
  via `tier_for_actions(kind, &[String]) -> PermissionTier`.
- `crates/starter-authz/src/instances.rs` — defines
  `InstancesProvider` trait, `InstancesQuery`, `InstancesPage`,
  `ResourceInstance`, `SubjectRef`, `ShareScope`, `PermissionTier`,
  `GrantSummary`, `EffectiveAcl`. Registry side-table:
  `InstancesRegistry { providers: HashMap<String, Arc<dyn InstancesProvider>> }`
  with `register(kind, provider)` + `get(kind)`. Held alongside
  the existing `ResourceRegistry` in router state.
- `crates/starter-authz/src/routes/instances.rs` — handler
  `list_instances(Path(kind), Query(InstancesQuery), State(...))`.
  Looks up provider; 404 if absent. Calls
  `provider.list(principal, tenant_id, query).await`. Admin-gated
  by the existing `admin_gate` layer.
- `rubix/crates/rubix-agent/src/boot/authz_instances.rs` —
  `DashboardPageInstancesProvider { store: Arc<dyn DashboardStore>, policy_store: Arc<dyn PolicyStore> }`.
  `list()` does (no N+1):
  1. `store.list_active(tenant_id, &ListFilter { ... })`.
  2. `policy_store.list_rules().await?` then filter in memory to
     `resource == "rubix.dashboard.page" && tenant_id matches`.
  3. Call `starter_authz::acl::summarise(rules, pages_iter)`.
- `packages/starter-ui-authz/src/panels/pages-tab.tsx` — table with
  columns Page / Access / Owner / Updated; debounced search;
  legacy badge; click row → drawer.
- `packages/starter-ui-authz/src/panels/page-detail-drawer.tsx` —
  read-only in G2. Owner, page_id, share-scope radios (disabled
  with tooltip), grants list (chips read-only).

### Files to edit
- `crates/starter-authz/src/lib.rs` — `pub mod acl; pub mod instances;`.
- `crates/starter-authz/src/routes/mod.rs` — `pub mod instances;`.
- `crates/starter-authz/src/routes/router.rs` — mount
  `GET /v1/authz/resources/:kind/instances`.
- `crates/starter-authz/src/routes/state.rs` — add
  `instances_registry: Arc<InstancesRegistry>` field.
- `rubix/crates/rubix-agent/src/boot/authz.rs` (lines 39–55 area)
  — after kind registration, build `InstancesRegistry`, register
  `"rubix.dashboard.page"` → provider, plumb into router state.
- `packages/starter-client-ts/src/endpoints/authz.ts` — append
  `listResourceInstances` method + types.
- `packages/starter-ui-authz/src/panels/authz-admin.tsx` — mount
  Pages tab under TenantDetail (both modes).
- `packages/starter-ui-authz/src/panels/index.ts` — re-exports.

### Tests
Backend:
- `acl::buckets_grants_picks_highest_tier`
- `acl::flags_legacy_rules_with_conditions`
- `acl::detects_tenant_share_scope_from_wildcard_subject`
- `acl::private_when_no_non_owner_grants`
- `routes::instances::lists_pages_for_tenant`
- `routes::instances::unknown_kind_returns_404`
- `routes::instances::respects_search_and_cursor`

Playwright: `access-pages-tab.spec.ts`.

### Risks
- `DashboardStore::list_active` may not accept search natively
  (check `ListFilter`). If not, filter in memory and document.
- Cursor format: if `ListFilter` lacks offset, ship in-memory
  cursor for v1; revisit when tenants pass ~1k pages.

---

## G3 — Grants API + page-drawer mutations (backend + frontend) — includes schema migration

### Migrations (both dialects)
- `crates/starter-authz/migrations/starter_authz_sqlite/0006_authz_rules_grants.sql`
- `crates/starter-authz/migrations/starter_authz_postgres/0007_authz_rules_grants.sql`

```sql
ALTER TABLE starter_authz_rules ADD COLUMN source TEXT NOT NULL DEFAULT 'manual';
ALTER TABLE starter_authz_rules ADD COLUMN resource_id TEXT;
CREATE INDEX idx_starter_authz_rules_grant_instance
  ON starter_authz_rules (resource, resource_id, tenant_id)
  WHERE source = 'grant';
```

Sqlite: split into two `ALTER` statements (one column each).

### Files to create
- `crates/starter-authz/src/grants.rs` — `GrantStore` wrapper around
  `Arc<dyn PolicyStore>` + reload hook. Methods: `create`, `delete`,
  `patch_tier`, `list`, `set_share_scope`. Tier→actions table for
  `rubix.dashboard.page` (View=["view"], Edit=["view","edit"],
  Manage=["view","edit","delete"]). Inserts: `source="grant"`,
  `priority=100`, `condition=NULL`,
  `role="team:<slug>" | "user:<sub>" | "*"`.
- `crates/starter-authz/src/routes/grants.rs` — handlers for the
  four routes + share-scope helper. CSRF + admin gate; `engine.reload()`
  after every store write.
- `packages/starter-ui-authz/src/panels/grants-combobox.tsx` —
  subject picker.

### Files to edit
- `crates/starter-authz/src/store/mod.rs` — extend `StoredRule`
  with `pub source: String` and `pub resource_id: Option<String>`.
- `crates/starter-authz/src/store/sqlite.rs` + `store/postgres.rs`
  — add the two columns to all SELECT/INSERT/UPDATE.
- `crates/starter-authz/src/engine.rs` (`roles_for`, lines 108–120)
  — append `team:<slug>` for each slug in `p.teams`.
- `crates/starter-authz/src/db_engine.rs` — verify role-resolution
  reuses `engine.rs::roles_for`; if it has its own, mirror.
- `crates/starter-authz/src/acl.rs` (from G2) — switch summariser
  to read `rule.resource_id` directly.
- `crates/starter-authz/src/lib.rs` — `pub mod grants;`.
- `crates/starter-authz/src/routes/mod.rs` — `pub mod grants;`.
- `crates/starter-authz/src/routes/router.rs` — mount the four
  grant routes + share-scope helper.
- `packages/starter-client-ts/src/endpoints/authz.ts` — append
  `createGrant`, `deleteGrant`, `listGrants`, `patchGrant`,
  `setShareScope` + types.
- `packages/starter-ui-authz/src/panels/page-detail-drawer.tsx` —
  wire mutations (Add / tier dropdown / Revoke / share-scope).

### Tests
- `grants::create::expands_edit_tier_to_view_plus_edit`
- `grants::create::writes_source_marker`
- `grants::create::role_team_slug_format`
- `grants::delete::removes_only_target_row`
- `grants::patch::tier_update_rewrites_actions_in_place`
- `grants::share_scope::tenant_writes_wildcard_subject_view_rule`
- `grants::share_scope::private_deletes_all_grant_rows`
- `engine::roles_for::synthesises_team_slug_from_principal_teams`
- E2E: grant + `POST /v1/authz/check` for member → Allow, non-member → Deny.

Playwright: `access-grants-drawer.spec.ts`, `access-grants-share-scope.spec.ts`.

### Risks
- `db_engine.rs` may cache its own role-resolution. Verify before
  editing.
- The existing `0006_bootstrap_admin_rule.sql` is postgres-only.
  Sqlite migrations stop at 0005, so `0006` is correct on sqlite.

---

## G4 — Team detail Permissions tab (frontend only)

### Files to create
- `packages/starter-ui-authz/src/panels/team-permissions-tab.tsx` —
  table with columns Resource / Kind / Tier / Granted by / Actions.
  Calls `client.listGrants({ subject: "team:" + slug })`. Classify
  rows: direct grant (source==="grant" and role==="team:<slug>"),
  tenant default (role==="*"), legacy rule (condition !== null).
  Revoke button only for direct grants.

### Files to edit
- `packages/starter-ui-authz/src/panels/authz-admin.tsx` (TeamDetail)
  — add `Permissions` `TabsTrigger` + `TabsContent` mounting the
  new tab, alongside existing Members tab.
- (optional) `rubix/frontend/src/lib/access-control.tsx` — recognise
  trailing `/permissions` segment after team slug; thread an optional
  `teamTab` field through `SelectedNode`. Backward-compatible default
  of `members`. Deferable.

### Tests
Playwright: `access-team-permissions.spec.ts`.

### Risks
- TeamDetail tab `defaultValue` is currently local state, not URL.
  G4 can ship without URL routing; defer URL-derived team tabs.
- `listGrants` response must include rule.source / condition / role
  raw so the UI can classify.
