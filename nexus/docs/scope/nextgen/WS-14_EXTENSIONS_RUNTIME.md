# WS-14 — Extensions Runtime: wire `starter-extensions` into nexus (host + lifecycle + cleanup)

> **Status:** Landed (first cut — backend mount + FE admin + Wave 3 host-methods) · **Wave:** 2 (backend mount + lifecycle) + 3 (capability host-methods) · **Owner:** _unassigned_
> **Depends on:** the `starter-extensions` kernel (already built — see §2) · NEXUS.md §7 (federation contract), §4 (secrets), §1 (RLS) · ROADMAP M2 "federation host"
> **Reshapes:** nothing — it's the missing *integration* of an existing subsystem. **Touches:** WS-10 (an extension can contribute query-kinds), WS-12 (enable/disable/install are auditable), WS-08 (an extension can contribute a datasource-kind).
> **Migration:** none in nexus's block range — `starter-ext-store-pg` ships its own `extensions_enablement` migration; nexus runs it in the metadata DB.
> **Verified:** re-grep the `starter-extensions` crate file:line claims below before building — the kernel is under active development and module layout may have shifted.
>
> **Origin:** the `starter-extensions` workspace (`crates/starter-ext-*`, `packages/starter-ext-ui` + `starter-ext-sdk-ts`). NEXUS.md §7 already names this as the federation model and ROADMAP M2 mandates standing up the host. The **frontend host is already partially wired into nexus-ui**; the **backend is not mounted into nexus-api at all** — that asymmetry is the core of this WS.

---

## 1. The idea in one paragraph

`starter-extensions` is a near-complete extension kernel: a manifest-driven registry (`starter-ext-host`), an OTP-style process **supervisor** that keeps process-flavour extensions running with restart/backoff/health + a process-group reaper (`starter-ext-supervisor`), an admin HTTP surface for list/detail/enable/disable/install/uninstall/cleanup with ETag-cached Module-Federation bundle serving (`starter-ext-server`), Postgres-backed enablement persistence (`starter-ext-store-pg`), and a Vite-library-mode federation host + remote SDK for the UI (`@nube/starter-ext-ui`, `@nube/starter-ext-sdk-ts`). **Almost none of the design problems remain — the work is integration.** nexus-ui already constructs the federation host and calls `bootstrapExtensions` against `GET /api/v1/extensions`, but nexus-api never mounts `starter-ext-server`, so those endpoints 404 and no remote can ever load. This WS **mounts the kernel into nexus-api** (admin routes + supervisor boot + cleanup providers + Postgres enablement store), **completes the frontend host loop** against the now-real endpoints, and **fills the handful of kernel stubs nexus actually needs** (caller extraction → capability host-methods). The payoff: a feature ships as a **file-drop/tarball-install extension** — backend (tools/REST/kinds/nodes) and frontend (a panel/slot component) — loaded, supervised, and cleanly removed without recompiling nexus.

---

## 2. What already exists (the kernel) — evidence

This is a *finish*, not a *build*. Inventory of `starter-extensions` (re-verify file:line before relying):

| Crate | What it already does | Key entry points |
|---|---|---|
| **`starter-ext-host`** | Manifest registry. `Loader::scan(root)` walks one level, parses each `block.yaml`, two-phase `validate_all()` + `commit()`; per-candidate errors isolated. Sealed `ExtensionRegistry` (HashMap, deterministic `list()`). | `Loader::scan`, `ExtensionRegistry`, `ExtensionRecord`, `Manifest` |
| **`starter-ext-spi`** | The contract. `Manifest` (`v:1`, `id` reverse-DNS, `version`, `runtime: builtin\|wasm\|process`, `supervision`, `capabilities`, `contributes`), `#[serde(deny_unknown_fields)]`. `Contributes`: `tools[] / rest[] / cli[] / grpc[] / workers[] / ui / i18n / nodes[] / skills[] / warehouse_templates[] / warehouse_tables[] / anomaly_rules[]`. Typed `Capability` enum (Secrets/HttpOut/Fs/WarehouseRead/Write/EventBus/Authz/Dashboard/…). | `Manifest`, `Contributes`, `Capability` |
| **`starter-ext-supervisor`** | *"The server add that keeps it running."* `Supervisor::start()` spawns process-flavour: init handshake (manifest content-hash verify), restart policy (`Always/OnCrash/Never` + intensity cap + exponential backoff/jitter), health pinging, `SIGTERM`→grace→`SIGKILL` shutdown, bounded `EventRing`. **Process-group reaper** (`reaper.rs`): child in own process group, pidfile per extension, boot-time `reap_stale_groups()` `killpg`s orphans from a prior crash (fixes tokio `kill_on_drop` only-direct-child / only-graceful gap). | `Supervisor::start`, `SupervisorHandle`, `reap_stale_groups`, `HostMethodHandler` |
| **`starter-ext-server`** | Admin surface + REST adapter, mountable into a `starter-server` `ServerBuilder`. `GET /extensions`, `GET /extensions/<id>`, `POST …/enable`, `DELETE …/<id>` (disable), `GET …/<id>/ui/*` (SHA-256 ETag + `304`), `GET …/<id>/events` (ring + SSE), `GET …/<id>/process` (live PID/stats), `POST /extensions/install` (gzip-tar multipart), `DELETE /extensions/<id>?purge=` (uninstall + cleanup), `GET …/<id>/cleanup` (dry-run). Mutations gated `Role::Admin`. Builtin REST/tool dispatch complete (streaming SSE/NDJSON, schema validation, auth). | `router(admin, options)`, `ExtensionAdmin`, `ExtensionAdminBuilder` |
| **cleanup (`starter-ext-server/cleanup.rs`)** | *"When removed, a clean-up is done."* `CleanupProvider { discover(), purge() }` trait, idempotent. Built-ins prepended: `EnablementRowProvider` (kills ghost DB row), `UiCacheProvider` (evicts ETag/byte cache), `I18nCacheProvider`. Consumer providers registered via `with_cleanup_provider()`. `?purge=true` runs all; dry-run via `discover()`. `CleanupKind`: WarehouseTable/EnablementRow/UiCache/I18nCache/Skill/Subscription. | `CleanupProvider`, `purge_cleanup()`, `ExtensionAdminBuilder::with_cleanup_provider` |
| **`starter-ext-store-pg`** | `PgEnablementStore: EnablementStore` over `PgPool`; `get/set/delete` + `set_as(actor,…)` for audit. Ships `migrations/0001_extensions_enablement.sql` (`extension_id PK, state CHECK(enabled\|disabled), updated_at, updated_by`). | `PgEnablementStore` |
| **`@nube/starter-ext-ui`** (FE host) | `ExtensionHostManager` (singleton negotiation: one React / one QueryClient / one zustand; major-mismatch = hard refusal), `ExtensionHostProvider`, `<ExtensionSlot>`, `bootstrapExtensions(host, {basePath})` (fetch enabled remotes, `import()` each `remoteEntry.js`, register slot contributions). | `ExtensionHostManager`, `bootstrapExtensions`, `ExtensionSlot` |
| **`@nube/starter-ext-sdk-ts`** (FE remote) | `ExtensionRemoteFactory` shape: `singletons` record + `init(handle)`; `registerExtensionContributions(handle, { components })` → named slots. | — |
| **examples** | `hello-process` (process runtime; one tool over MCP/REST). `notes` (tools + rest + cli + ui in one bundle). `hello-ui` (builtin + `contributes.ui` → `HelloPanel` in `sidebar`). | `examples/{hello-process,notes,hello-ui}` |

**Known kernel stubs** (only some matter to nexus):
1. **Caller extraction** — `CapabilityFactory` receives `caller: None`; HTTP→principal extraction not wired. *(Matters: capability host-methods can't be tenant-scoped without it.)*
2. **Host-method handlers** — `NotImplementedHandler` returns error for every `dashboard.*`/`authz.*`/`warehouse.*` call. *(Matters if nexus extensions call back into nexus.)*
3. **REST dispatch for process/wasm** — `NotWiredDispatcher` → `503`; only **builtin** REST dispatch works today. *(Matters if a process extension contributes REST; tools-over-stdio do work.)*
4. **Registry-URL install** — JSON-body install is `501`; only tarball-upload installs. *(Defer — tarball is enough for v1.)*
5. **No hot-mount after seal** — installed extensions go live on **next boot** (registry sealed at startup). *(Accept for v1; surface `pending_restart: true`, which the server already returns.)*

---

## 3. The actual gap in nexus — evidence

- **Backend: not mounted.** `grep starter-ext nexus/backend` → **nothing**. nexus-api has no `starter-ext-*` dependency, builds no `ExtensionAdmin`, mounts no `/extensions` routes, boots no supervisor, registers no cleanup providers. ([nexus/backend/crates/nexus-api/src/main.rs](../../../backend/crates/nexus-api/src/main.rs), [serve.rs](../../../backend/crates/nexus-api/src/serve.rs) — no extension wiring.)
- **Frontend: wired but dangling.** nexus-ui builds the host and calls bootstrap, but against endpoints that 404:
  - [nexus/ui/src/app/providers.tsx](../../../ui/src/app/providers.tsx) wraps the tree in `ExtensionHostProvider`.
  - [nexus/ui/src/extensions/host.tsx](../../../ui/src/extensions/host.tsx) constructs `ExtensionHostManager` (publishes `__rubix*` React globals for the in-repo `com.nubeio.ce` remote; registers react/react-dom/react-query/zustand singletons).
  - [nexus/ui/src/extensions/AutoLoader.tsx:20](../../../ui/src/extensions/AutoLoader.tsx#L20) calls `bootstrapExtensions(host, { basePath: "/api/v1/extensions" })` after auth.
  - [nexus/ui/src/app/AppSidebar.tsx:87,93](../../../ui/src/app/AppSidebar.tsx#L87) renders `<ExtensionSlot id="sidebar-nav" />` and `<ExtensionSlot id="sidebar" />`.
  - **All of this is inert** until nexus-api serves `GET /api/v1/extensions` and `GET /api/v1/extensions/<id>/ui/*`.

**So the central task is one seam:** mount `starter-ext-server` into nexus-api's router under `/api/v1/extensions`, boot the supervisor + Postgres enablement store at startup, and register nexus-specific cleanup providers. Everything downstream (FE host loading, install/uninstall, cleanup) then works because the kernel already implements it.

---

## 4. Proposed design for nexus

### 4.1 Backend mount (Wave 2 — the high-value seam)

In nexus-api `main.rs` / `serve.rs`, beside the existing subsystem wiring:

1. **Add deps:** `starter-ext-server`, `starter-ext-host`, `starter-ext-supervisor`, `starter-ext-store-pg`, `starter-ext-spi` to `nexus-api/Cargo.toml` (all workspace deps via the parent `starter` workspace).
2. **Discover + load at boot:** `Loader::scan(&cfg.extensions_dir)` (default `./extensions`, env `NEXUS_EXTENSIONS_DIR` — mirror the WS-10 `NEXUS_KINDS_DIR` pattern, *including the dev-CWD + Docker-copy lesson from WS-10*: the dir must resolve under `cd backend && cargo run` and be `COPY`'d into the runtime image). `validate_all()` + `commit()` + `seal()`.
3. **Reap orphans first:** call `reap_stale_groups(pidfile_dir)` **before** spawning anything, so a prior crash leaves no orphaned grandchildren (the supervisor memory: process groups + `killpg` + boot pidfile reaper).
4. **Build `ExtensionAdmin`** via `ExtensionAdminBuilder`:
   - `EnablementStore` = `PgEnablementStore::new(metadata_pool)` (run its migration in `bootstrap::migrate_all`).
   - `SupervisorFactory` = `DefaultSupervisorFactory::with_pidfile_dir(cfg.ext_pidfile_dir)`.
   - cleanup providers: the three built-ins are automatic; **register nexus providers** for any extension-owned nexus state — at minimum a **query-kind cleanup provider** (delete rows in `nexus_query_kinds` an extension contributed, WS-10) and a **datasource-kind provider** (WS-08) once those exist. Keep the provider list the single place extension-owned cleanup is declared.
   - optional `PostInstallHook` for warehouse DDL if an extension declares `warehouse_tables[]` (rubix uses this).
5. **Spawn supervisors** for enabled process-flavour extensions (the factory returns `Ok(None)` for builtin/wasm).
6. **Mount the router:** `starter_ext_server::router(admin, options)` merged into nexus-api's router under `/api/v1/extensions`, behind the **same `Authenticator` middleware** nexus uses, with mutations gated `Role::Admin` (the server already applies `with_role`). Confirm the `with_principal`/`with_role` middleware the crate expects matches nexus's `starter-server` stack.
7. **Graceful shutdown:** on process exit, shut down all `SupervisorHandle`s (`SIGTERM`→grace→`SIGKILL`) so no child outlives nexus.

### 4.2 Frontend host completion (Wave 2 — small)

The host is wired; this is verification + the missing UX, not new architecture:
- **Confirm the loop** end-to-end: with the backend mounted, `bootstrapExtensions` lists enabled UI extensions and `import()`s each `remoteEntry.js` from `GET /api/v1/extensions/<id>/ui/*`. Verify the in-repo `com.nubeio.ce` remote mounts into `<ExtensionSlot id="sidebar">` **unchanged** (NEXUS.md §7 hard requirement) — the `__rubix*` globals are already published.
- **Admin UI** (the genuinely-missing FE piece): an **Extensions** management page (under Manage, beside Access/Audit) listing extensions with state/restart-count/violations, and enable/disable/install(upload tarball)/uninstall(+purge, with the **dry-run cleanup manifest shown before confirm**) actions hitting the admin endpoints. Mirror the existing Access/Audit page shape; the dry-run-before-purge step is the safety UX the cleanup endpoint is designed for.
- **Slot coverage:** decide which nexus regions are extension slots (sidebar nav, dashboard panel-type registry, a settings slot). The panel-type-as-extension path (NEXUS.md §7: "a Nexus panel type can ship as an extension") is the strategic one — scope it but it can trail.

### 4.3 Capability host-methods (Wave 3 — fill the stubs nexus needs)

Only needed once a nexus extension calls **back into** nexus (read a dashboard, check authz, read the warehouse). For tools/REST/UI-only extensions, skip.
- **Wire caller extraction:** thread the authenticated `Principal` from the HTTP request into `CapabilityFactory` (today `caller: None`) so a capability call is tenant-scoped.
- **Implement `HostMethodHandler`** for the methods nexus exposes, gated by the manifest's declared `Capability` set: `warehouse.read` (→ the WS-10 kind dispatch / query path under the caller's tenant), `authz.check` (→ `starter-authz`), `dashboard.read` (→ dashboard store). Each enforces the capability allowlist **and** the tenant predicate — an extension capability is never broader than the caller's grants.

### 4.4 What an extension looks like in nexus (the deliverable shape)

A nexus feature shipped as an extension = one bundle dir / tarball with a `block.yaml`:
- **Backend contributions:** `tools[]` (MCP/REST), `rest[]`, `nodes[]` (flow node kinds), `warehouse_templates[]` (WS-10 query-kinds!), `warehouse_tables[]`. Runtime `builtin` (in-tree crate) or `process` (supervised binary).
- **Frontend contributions:** `ui.entry: ui/remoteEntry.js` exposing components into named slots.
- **Removal:** `DELETE /extensions/<id>?purge=true` stops the supervisor, deletes the bundle, and runs every cleanup provider — DB rows, caches, warehouse tables, contributed kinds — idempotently. **Additive by install, removable by delete, zero nexus recompiles.**

---

## 5. Relationship to other workstreams

- **WS-10 (Kinds)** — the cleanest backend contribution: an extension's `warehouse_templates[]` *are* query-kinds. Two integration choices (settle in §8): (a) the kind registry gains an **extension source** (a third source beside file-pack + tenant-DB), or (b) install materializes contributed kinds into the existing store. Either way, the **query-kind cleanup provider (§4.1.4) removes them on uninstall.**
- **WS-08 (Datasources)** — an extension can contribute a **datasource-kind** (declarative connector). Same pattern: contribute on install, clean up on remove.
- **WS-12 (Audit/Undo)** — enable/disable/install/uninstall are tenant/admin mutations; record them via the changelog. `PgEnablementStore::set_as(actor,…)` already carries the actor — wire it to the audit ledger. (Extensions are global config, not per-tenant rows, so this is an **audit-only** entry, not a `Reversible` resource — note that, like the file-pack kinds in WS-10 C6.)
- **WS-07 / WS-06** — extension-contributed alert channels and flow nodes plug in through `contributes.{anomaly_rules,nodes}`; out of scope here beyond ensuring the registry exposes them.

---

## 6. Scope (this workstream)

1. **Mount the backend** (§4.1): deps + boot-time `scan/validate/commit/seal`, `reap_stale_groups` first, `PgEnablementStore` + its migration, `DefaultSupervisorFactory` with pidfile dir, spawn enabled process extensions, mount `starter_ext_server::router` under `/api/v1/extensions` behind nexus auth + `Role::Admin`, graceful supervisor shutdown.
2. **Config + deploy** (§4.1.2): `NEXUS_EXTENSIONS_DIR` + `ext_pidfile_dir`, **applying the WS-10 dev-CWD + Dockerfile-copy lesson** so the dir resolves in `make dev-be` and in the Fly image.
3. **Nexus cleanup providers** (§4.1.4): a query-kind provider (WS-10) and the seam for a datasource-kind provider (WS-08); built-in row/cache providers are automatic.
4. **Complete the FE loop** (§4.2): verify `bootstrapExtensions` loads remotes against real endpoints; confirm `com.nubeio.ce` mounts unchanged into the existing slots.
5. **Extensions admin page** (§4.2): list + enable/disable + install(upload) + uninstall(**dry-run cleanup → confirm → purge**), mirroring Access/Audit.
6. **Capability host-methods** (§4.3, Wave 3): caller extraction + `HostMethodHandler` for `warehouse.read`/`authz.check`/`dashboard.read`, each capability-gated and tenant-scoped.
7. **One end-to-end example** in nexus: install `hello-ui` (or a nexus-flavoured panel extension) → see it in a slot; install `notes` (tools+rest) → call its REST route through the adapter; uninstall+purge → confirm clean.

## 7. Acceptance criteria
- [ ] `GET /api/v1/extensions` returns the registry (was 404); the FE `AutoLoader` loads enabled UI remotes and the in-repo `com.nubeio.ce` mounts **unchanged** in `<ExtensionSlot id="sidebar">`.
- [ ] A **process-flavour** extension is spawned at boot, restarts per its `supervision` policy on crash, and `GET …/<id>/process` reports a live PID.
- [ ] Killing nexus does **not** leave orphaned extension grandchildren (reaper verified: a stale process group from a prior run is `killpg`'d on next boot).
- [ ] `POST /extensions/install` (tarball) lands a bundle, persists `Enabled`, returns `pending_restart: true`; after restart the extension is live.
- [ ] `DELETE /extensions/<id>?purge=true` stops the supervisor, removes the bundle, and runs **all** cleanup providers — including the **nexus query-kind provider**, so a kind the extension contributed (WS-10) is gone and a re-list no longer shows it. Re-purge is idempotent (no error on already-clean).
- [ ] The dry-run cleanup manifest (`GET …/<id>/cleanup`) is shown in the admin UI **before** a purge is confirmed.
- [ ] Enable/disable/install/uninstall are gated `Role::Admin` and recorded in the audit ledger with the acting principal.
- [ ] (Wave 3) A capability-gated host-method (`warehouse.read`) runs under the **caller's tenant** and refuses a table outside the extension's declared `WarehouseRead` allowlist.
- [ ] Backend builds with the extension crates mounted; existing nexus tests stay green; the extension dir resolves in both `make dev-be` and the Docker image.

## 8. Out of scope (defer / hand off)
- **Out-of-repo extension *security*** (allowlist / signature / checksum-pin / CSP on `remoteEntry.js`) — NEXUS.md §7 says this **must precede loading any out-of-repo remote**, but v1 loads only in-repo `com.nubeio.ce`, so it trails into a dedicated security pass. Ship in-repo-only first.
- **Hot-mount after seal** — accept `pending_restart` + restart-to-activate for v1 (the kernel is sealed-by-design).
- **Registry-URL / marketplace install** — tarball upload only (the JSON path is `501` in the kernel).
- **WASM-flavour + process-flavour REST dispatch** — kernel `NotWiredDispatcher` is `503`; v1 nexus extensions use **builtin** REST or **tools-over-stdio**. Wire process/wasm REST when a real need appears.
- **cgroups / rlimits / supervisor groups** — kernel v0.2; not required for nexus v1.
- **Panel-type-as-extension** registry — scoped (§4.2) but may trail the first cut.

## 8.5 What landed (first cut) — status notes

**Backend mount (§4.1) — done.** `nexus-api/src/extensions/` owns the integration:
- `boot.rs` — reap orphans → scan/validate/commit/seal (`NEXUS_EXTENSIONS_DIR` pack + writable installs dir, both scanned) → materialise contributed kinds → spawn enabled process supervisors → assemble `ExtensionAdmin`. Supervisors drain on graceful shutdown (`ExtensionAdmin::shutdown_all`, added to the kernel).
- `router.rs` — kernel `router_with_auth` nested under `/api/v1`, merged as a **sibling** of the authz/tenants routers (the kernel applies its own `with_principal` → `with_role(Admin)`; nesting it inside nexus's product principal layer would double-run the layer).
- Migrations: `extensions_enablement` runs as the namespaced `ext_store` source (the store-pg crate now exports its `MIGRATOR`); `nexus_extension_query_kinds` is nexus migration `1801`.

**Kind contribution (§5 Q1) — settled as the third-source approach.** Extension `warehouse_templates[]` land in a **global, non-RLS** `nexus_extension_query_kinds` table keyed by `(extension_id, name)`; the dispatcher resolves file-pack → **extension** → tenant overlay (global precedes tenant, matching file-pack precedence). Contributions pass the **identical lint** as file-pack/tenant kinds before persisting. The in-memory `AppState.extension_kinds` registry is built at boot (sealed-by-restart, like the file pack — consistent with `pending_restart`). Cleanup = `QueryKindCleanupProvider` (delete by `extension_id`, idempotent); install-time persistence = `QueryKindPostInstall` hook.

**Audit (§5 Q4) — settled, with a small kernel addition.** The kernel's enable/disable handlers did not extract the principal at all, so no consumer-side decorator could attribute the actor. Added an additive `AuditSink` seam to `starter-ext-server` (`with_audit_sink`, no-op default; enable/disable/install/uninstall notify it with the acting `Principal`). nexus's `NexusExtensionAudit` records audit-only `Op::Custom` rows into `nexus_changes` under the acting admin's tenant (sentinel `_global` for tenant-less super-admins) via `nexus_store::changelog::record_audit`.

**Wave 3 host-methods (§4.3) — done.** `NexusHostMethods` implements `authz.check` (policy engine, caller's tenant), `dashboard.read` (tenant-clamped + `view`-gated), and `warehouse.query` (resolves a *global* kind — file-pack or extension-contributed, never the tenant overlay — and runs it with `$caller_tenant_id` bound to the caller). Installed via `WithHostMethodsFactory`; a caller without `tenant_id` is a hard deny. Caller extraction inside the kernel (`_meta.caller` threading) was already present at the supervisor layer; nexus consumes it as-is.

**FE (§4.2) — done.** `features/extensions/ExtensionsPage.tsx` (admin-gated, mirrors Audit): list with state/restart/violations badges, enable/disable switch, restart, tarball install dialog, and uninstall with the **dry-run cleanup manifest shown before purge confirm**. Routed at `/extensions`, linked in the sidebar Manage area.

**E2E example — `com.nexus.hello`** (in-repo pack at `nexus/backend/crates/nexus-api/extensions/`): builtin flavour contributing two query-kinds (`.ping`, `.echo`) + a hand-authored zero-build `remoteEntry.js` exposing `HelloPanel` into `sidebar`. A unit test (`extensions::contribute::tests`) drives the kernel loader + lint + dispatcher resolution against the shipped bundle.

**Config/deploy (§4.1.2) — done.** `NEXUS_EXTENSIONS_DIR` (read-only pack), `NEXUS_EXTENSIONS_INSTALLS_DIR` + `NEXUS_EXTENSIONS_PIDFILE_DIR` (writable, default under the pack / `.nexus-ext` in dev). Makefile dev vars + Dockerfile COPY + `/data` runtime dirs, mirroring the WS-10 lesson.

**E2E verified against a running stack** (db + seed + `make dev-be` + vite):
`make ext-hello-test` PASSes (list → detail → ui bytes 200+ETag/304 → both
contributed kinds run); unload/load/pack/install(upload)/cleanup-preview/purge
all exercised — purge removed the uploaded copy + kinds + enablement rows,
**left the in-repo pack intact** (`will_delete: false` via the new
`KeepDevSource` guard), re-purge idempotent, audit ledger carries
enabled/disabled/installed/uninstalled with the acting principal, and the next
boot re-materialised the kinds. Browser (Playwright headless Chromium): login →
federation load → `com.nexus.hello` panel mounts in the sidebar slot rendering
its own kind's result; the Extensions admin page lists the bundle. Console
clean. Three live bugs found + fixed during this pass (regression-tested):
1. `ExtensionRegistry::install` **replaces** records — committing the pack and
   installs scans separately wiped the pack; both roots now collect into one
   commit (installs-last wins an id clash).
2. The WS-10 query binder scanned `$tokens` inside SQL **comments, string
   literals, and dollar-quoted bodies** — a kind whose doc header merely
   mentioned `$caller_tenant_id` 4xx'd at bind time. The scanner is now
   comment/string/dollar-quote-aware (5 new binder tests).
3. The kernel's uninstall deleted any registry record's `bundle_dir` — for an
   in-repo pack bundle that meant `remove_dir_all` on repo source. The
   documented-but-unimplemented installs-tree sanity check now exists
   (`KeepDevSource`: outside installs ⇒ never deleted, reported
   `will_delete: false`); bundle removal also tolerates already-gone dirs.

**Follow-ups (not in this cut):**
- The in-repo `com.nubeio.ce` remote is a rubix bundle and was not exercised in nexus.
- Extensions entry in the server-seeded **nav tree** needs the `StaticRoute` enum + codegen extended (the sidebar link uses the static-menu pattern for now).
- Datasource-kind cleanup provider (WS-08) — the seam exists (`with_cleanup_provider`), no provider yet.
- Process-flavour e2e (restart policy, reaper, live PID) is kernel-tested but not exercised by a nexus integration test; needs a process-flavour fixture bundle.
- Out-of-repo remote security (§8) still trails as its own pass before any non-in-repo `remoteEntry.js` loads.

## 9. Open questions to settle in Wave 0
1. **Kind contribution path (§5):** extension as a *third kind source* in the WS-10 registry, vs. *materialize-on-install* into `nexus_query_kinds`. Recommend the **third-source** approach (keeps the file-pack/tenant-DB/extension boundaries clean; cleanup just drops the source) — confirm against the WS-10 dispatch shape.
2. **Middleware match:** does `starter-ext-server`'s expected `with_principal`/`with_role` middleware compose with nexus's `starter-server` auth stack as-is, or need a thin adapter? (The crate's Cargo comment flags an "adapter dependency arrow" — verify.)
3. **Extension dir hosting:** in-repo `./extensions` only for v1, or also a writable install dir for uploaded tarballs (the `installs_dir` the kernel supports)? Recommend both: a read-only in-repo pack **and** a writable uploads dir, mirroring WS-10 (a)+(c).
4. **Audit shape:** confirm enable/disable/install/uninstall record as **audit-only** changelog entries (not `Reversible` resources), consistent with global-config kinds (WS-12).
5. **Tenant scope of extensions:** are extensions **global** (one registry for the deployment) or **per-tenant**? The kernel's enablement store is keyed by `extension_id` only (global). Recommend **global install, admin-gated** for v1; per-tenant enablement is a later phase if needed.
