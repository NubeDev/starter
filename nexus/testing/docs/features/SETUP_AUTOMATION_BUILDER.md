# Feature: Setup / Automation Builder

> Status: **BUILT + WIRED + LIVE** (crates, stores, run/resume engine,
> REST/SSE/MCP surfaces, authz, the `$caller_team_ids` host token, and the
> extension seam). **Mounted in `nexus-api`** as of the setup-builder wiring:
> `/api/v1/setup/*` serves on the live server, the demo's bundled template is
> imported at boot, and a barcode → run executes its steps **in the extension
> child** over the `ProcessNodeProxy` flow-node bridge (see
> [§7 What's live](#7-whats-live)). `make -C nexus/extensions/com.acme.devices
> test` passes end-to-end against a running API.
> Spec: [`DOCS/setup-automation-builder.md`](../../../../DOCS/setup-automation-builder.md).
> Worked demo: [`nexus/extensions/com.acme.devices`](../../../extensions/com.acme.devices).

## The idea

A **template-driven automation builder**. An author composes a parameterized
multi-step automation once (YAML or, later, a visual builder); any authorized
user — including a mobile app scanning a barcode — runs it with a small input
form. Long runs **stream per-step progress** and **resume from the exact step
they failed on**. The generic machinery (engine, checkpoints, REST/MCP, authz)
is core; the **custom steps and templates plug in through an extension**.

```
 scan barcode ─▶ POST /setup/templates/com.acme.add-device/run
                    │  validate input · seed trusted identity slots · FlowRunner::start
                    ▼
              202 { run_id }            ◀── returns in ms, nothing blocks on completion
                    │
       GET /setup/runs/{id}/events (SSE)
                    │  step: device.create … step: sensor.register …
                    ▼
            register-sensor FAILS  ─▶  run halts: Failed + resumable + cursor
                    │
       POST /setup/runs/{id}/resume   ─▶  replays checkpoint, re-enters AT the cursor
                    ▼
              RunCompleted { device_id }   (device.create is idempotent — no double-create)
```

It is almost entirely **composition of primitives that already exist** —
`starter-flow` (engine, checkpoints, `FlowEvent` broadcast), `starter-authz` +
`starter-auth-users` (principals/teams/RBAC), the extension SDK. The net-new
code is a thin Template/Run domain plus its surfaces.

---

## 1. The pieces (crate map)

| Crate | What it owns |
|-------|--------------|
| `starter-setup-spi` | Domain: `Template`, `SetupRun`, `SemVer`, `TemplateStore` + `SetupRunStore` traits, the YAML `TemplateEnvelope`, the reserved trusted-identity slot names. |
| `starter-setup` | Run service (validate → seed → launch → project progress), YAML import/export, **resume**, authz registration + team check, REST/SSE surface (`rest` feature), MCP tools (`mcp` feature), the **extension import path** (`extension` module). |
| `starter-store-sqlite` / `-postgres` | `SqliteTemplateStore`/`SqliteSetupRunStore` (+ Pg twins) behind the default-off `setup` feature; a `setup` migration source (`setup_templates` + `setup_runs`). |
| `starter-flow` | The engine. P1a added one opt-in policy (below). |

### The four phases that matter for behaviour

- **P0 — domain + storage.** Catalog keyed `(tenant_id, id, version)` with a
  `'__global__'` sentinel for extension-provided templates (so two tenants
  installing the same extension template don't collide).
- **P1 — run service + crash recovery + trusted identity.** Instant launch,
  progress projector, `caller_user_id`/`caller_team_ids`/`caller_tenant_id`
  seeded **from the verified `Principal`** onto reserved slots.
- **P1a — fatal-failure → terminal-resumable (engine work).** The headline
  "resume from the failed step" is *new* engine behaviour, not free (see §3).
- **P3a — `$caller_team_ids` host token.** The one shared-query-internals change
  that unblocks team/site-scoped reusable pages (see §6).

---

## 2. Trusted identity — the security boundary (DOCS §9)

The run service seeds three reserved slots from the **verified** `Principal` at
`FlowRunner::start`:

```
caller_user_id    ← Principal.subject
caller_team_ids   ← Principal.teams      (JSON array)
caller_tenant_id  ← Principal.tenant_id
```

A custom node reads them like any slot. The rules, enforced in code:

- A template's `input_bindings` **may never target** a reserved slot name —
  rejected at import *and* at run (`SetupError::InvalidBinding`). So form input
  can never overwrite identity.
- Identity is **host-bound, never client-supplied.** An installer cannot spoof
  which site/owner a device is tagged to.

The `device_create` node in the demo reads `caller_user_id` from the seeded slot,
never from the form.

---

## 3. Resume — how "continue from the failed step" actually works (DOCS §8)

This is the subtle part. Two distinct recovery modes:

- **§8a crash recovery (pre-existing).** Process dies mid-run → the run's row has
  `finished_at IS NULL` → `list_open()` finds it on boot → replay the latest
  checkpoint → propagation continues.
- **§8b step-failure resume (NEW — P1a).** In the base engine a node error is
  **not terminal** — the propagator emits `NodeFailed` and keeps going. That's
  wrong for an automation. P1a added an **opt-in** policy:

  - `PropagatorConfig::halt_on_node_failure` (default `false`, so every other
    flow consumer is byte-identical). The setup run service turns it **on**.
  - On a node `Err` with the flag set: the propagator **always writes a terminal
    checkpoint** (even with no writes this tick, so resume has a row to load),
    emits `RunFailed`, and stops. The `setup_runs` row goes
    `Failed + resumable=true + failed_node=<cursor>`.
  - **Resume re-fires the cursor.** `FlowRunner::resume` replays the checkpoint
    (the R3 idempotent short-circuit absorbs no-op writes), then the service
    **force-seeds** (`WriteSlotOpts::forced()`) the cursor node's trigger slot
    from its current store value — a plain re-seed would be suppressed by the
    short-circuit and never re-trigger. Only the cursor node re-runs.

**Idempotency is the node author's contract (DOCS §8c).** Because resume may
re-enter a partially-completed step, any node that touches an external system
must be idempotent on a natural key. The demo's `device_create` derives its
`device_id` as a *pure function of the barcode* — same barcode → same id → no
second device.

---

## 4. The surfaces (DOCS §11)

REST for humans/mobile, MCP for AI; both share the same `RunService`.

### REST (`/api/v1/setup`, `rest` feature)

| Method | Path | Action |
|--------|------|--------|
| GET  | `/setup/templates` | list (nav-filtered) |
| GET  | `/setup/templates/{id}` | fetch (`?format=yaml` to export) |
| POST | `/setup/templates` / `/setup/templates/import` | create / YAML import |
| POST | `/setup/templates/{id}/run` | **launch → 202 { run_id }** |
| GET  | `/setup/runs` / `/setup/runs/{id}` | list / snapshot |
| GET  | `/setup/runs/{id}/events` | **SSE progress** |
| POST | `/setup/runs/{id}/resume` | **continue from failure** |
| POST | `/setup/runs/{id}/cancel` | cancel |

### MCP (`mcp` feature)

`setup.list_templates` · `setup.run_template { template_id, input }` ·
`setup.run_status { run_id }` · `setup.resume_run { run_id }`. Identity comes
from the MCP transport's verified principal (`current_principal()`), never input.

---

## 5. Authz — the two-layer check (DOCS §10)

1. **Generic authz** registers `setup.templates` + `setup.runs` resource kinds
   (tenant-scoped, owner-owned) and the default rules (writers manage their own
   templates; the coarse `run` gate; launchers read/resume/cancel their own runs).
2. **Setup-layer team check (Rust).** The per-template `allowed_teams` predicate
   **cannot** be an authz condition — the condition engine sees only
   `object.{kind,id,owner,tenant}`, never `allowed_teams`. So after the generic
   gate passes, the run handler loads the template and asserts
   `allowed_teams.is_empty() || principal.teams ∩ allowed_teams ≠ ∅`, plus a
   tenant backstop. This is `starter_setup::authz::team_check`.

### 5a. CSRF on mutating routes (platform-wide)

The setup mutations (`/run`, `/resume`, `/cancel`, template create/import/delete)
inherit the **double-submit CSRF guard** that now wraps every cookie-authenticated
mutation on the nexus product surface — not a setup-specific bolt-on. The guard
(`starter_server::auth::csrf_guard`, wrapped inside `with_principal` in
`serve::assemble` over the product / authz / tenants / extensions / setup
routers) requires a `POST`/`PUT`/`PATCH`/`DELETE` made with the `starter_session`
cookie to echo the `starter_csrf` cookie back as `X-CSRF-Token`. **Bearer-token
API clients and safe methods are exempt** (no ambient cookie to forge). The TS
client (`@nube/starter-client-ts`) auto-attaches the token on mutating methods in
`fetchJson`/`fetchVoid`, so the SPA needs no per-call change. Before this, the
nexus product surface enforced no CSRF at all (a gap surfaced while wiring setup);
closing it uniformly was the long-term-correct fix.

---

## 6. `$caller_team_ids` — the one shared-internals change (P3a)

The query binder already had `$caller_tenant_id` / `$caller_user_id` (host-bound,
un-spoofable). P3a added `$caller_team_ids`, bound from `Principal.teams` as a
`text[]` for `WHERE site_team = ANY($caller_team_ids)` — the "my team's rows"
filter tier between own-row and whole-tenant.

It touches **three** places (a live e2e caught the third):

- `nexus-store::query::bind` — `HostTokens.caller_team_ids`, the new
  `SqlValue::TextArray`, expansion + rejection-in-input.
- `nexus-store::query::request` — `QueryIdentity.teams`, bound from the principal.
- **`nexus-api/src/kinds/lint.rs`** — the boot-time query-kind linter has its
  **own** `HOST_TOKENS` allowlist (separate from the binder). It was `[2]`; now
  `[3]`. Without this, a contributed query-kind using `$caller_team_ids` is
  **rejected at boot** ("not a host token"). This is the gap the demo surfaced.

Also: the nexus query **cache key** now folds in `caller_user` + `caller_teams`
(sorted) — it previously keyed only on tenant, so identity-scoped queries would
have served one user's rows from another's cached entry.

---

## 7. What's live

**Built + unit/integration-tested (all green):**

- `starter-setup*` crates, both stores, the run/resume engine work, REST/SSE/MCP
  surfaces, authz, the extension import path. Key tests:
  - `starter-setup/tests/run_and_resume.rs` — fail → halt → resume-from-cursor → complete.
  - `starter-setup/tests/http_e2e.rs` — the whole barcode story over a real axum app.
  - `starter-setup/tests/extension_import.rs` — imports the **real** demo bundle template.
  - `starter-setup/tests/authz_team_check.rs` — the team check + tenant backstop.
  - `nexus-store` `query_bind` — `$caller_team_ids` binds `text[]`, rejected in input.

**Mounted in `nexus-api` and verified live against a running server:**

- `com.acme.devices` **loads clean** (`state: validated, failure: null,
  runtime: process`) and its supervised child **spawns and runs** (parity with
  `com.nexus.hello`). The detail endpoint's `workers: []` is the *periodic-worker*
  adapter, NOT a child-liveness signal — process children run regardless.
- Its **UI bundle serves** (HTTP 200 + ETag + 304 revalidation).
- Its **`site_checkout` query-kind is registered** (contributed query-kind count
  went 4 → 5 after the lint fix). It selects the host-bound `$caller_tenant_id` /
  `$caller_team_ids`, so it only binds on a **principal-bearing** query path
  (`POST /api/v1/datasources/{id}/query`); the ad-hoc `POST /api/v1/query` runs
  with `QueryIdentity::default()` (no principal) and correctly returns 400
  "needs host token" — the guard is enforced, not a bug.
- **`/api/v1/setup/*` serves** (404 gone). The boot wiring builds the
  `RunService` over the Postgres `PgTemplateStore` + `PgSetupRunStore` (behind
  the `setup` migration source), a `FlowRunner` with
  `SetupEngine::runner_config()` (§8b halt policy) + the durable `PgRunStore` SPI
  run store, registers the authz specs, imports each enabled extension's
  `contributes.setup_templates[]` into the global catalog, and mounts the router
  under the principal layer.
- **The barcode → run executes its steps in the extension child.** Each
  extension's `contributes.nodes[]` is bridged at boot by a `ProcessNodeProxy`
  (FLOW-NODES slice B) into the shared `NodeKindRegistry` the setup engine uses,
  so `device_create` / `sensor_register` route `flow.node.invoke` to the running
  child. A live `POST /setup/templates/com.acme.add-device/run` returns
  `202 { run_id }` and the run reaches `status: completed, progress: 2/2`, with
  trusted identity (`owner`/`tenant_id`/`team`) seeded from the verified
  principal — not the form.

**Where the wiring lives (nexus-api):**

- `Cargo.toml` — `starter-setup` (`rest`,`mcp`), `starter-flow`,
  `starter-flow-spi`, `starter-ext-flow`; `starter-store-postgres` gains
  `setup`,`flow`.
- `src/setup/mod.rs` — `build_service`, `register_authz`,
  `import_extension_templates`.
- `src/extensions/contribute_nodes.rs` + `src/extensions/boot.rs` — the
  generic `ProcessNodeProxy` bridge; `ExtensionRuntime.flow_node_kinds`.
- `src/bootstrap.rs` — the `setup` migration source.
- `src/identity.rs` — `register_specs` into the authz registry.
- `src/serve.rs` + `src/main.rs` — the `/api/v1/setup/*` sibling mount.

**Resume note.** The §8b resume-from-cursor loop is proven by
`run_and_resume.rs` in-crate and the live `/setup/runs/{id}/resume` route is
mounted and responds correctly (a completed run returns a clean 400
`not resumable`). The demo's `device_create` is idempotent and succeeds, so a
live *failure*→resume needs a node rigged to fail once; the in-crate test is the
canonical proof of the full loop.

---

## 8. The demo extension: `com.acme.devices`

A **process-flavour** bundle (modelled on `com.nexus.hello`) that exercises every
seam the builder adds. Layout:

```
nexus/extensions/com.acme.devices/
├── block.yaml                    # manifest — see contributes below
├── process/src/main.rs           # the supervised child (acme-devices-extension)
├── templates/add-device.yaml     # the bundled setup automation (envelope)
├── schemas/*.json                # node-kind / tool I/O schemas
├── kinds/site_checkout.sql       # verify-page query-kind ($caller_team_ids)
├── ui-src/ + ui/remoteEntry.js    # shadcn "Provision device" panel + nav (built)
└── Makefile                      # build / pack / install / load / test
```

`block.yaml` `contributes`:

- **`nodes[]` + `tools[]`** — `com.acme.devices.device_create` and
  `com.acme.devices.sensor_register`. Declared as flow node-kinds (the real setup
  mechanism — the host inserts a `ProcessNodeProxy` routing `flow.node.invoke` to
  the child) *and* as tools (which the SDK's `ToolHandlers` fully serves today).
  Both **idempotent on a natural key** (DOCS §8c).
  > **Namespace gotcha:** the ext SDK's R4 rule requires node/tool ids to be
  > *descendants* of the extension id. `com.acme.device.create` is **rejected**;
  > it must be `com.acme.devices.device_create`.
- **`setup_templates[]`** — `templates/add-device.yaml`, imported into the
  `TemplateStore` on enable with `source = Extension`. This is the net-new
  manifest field (`ContributeSetupTemplate { id, file }`).
- **`warehouse_templates[]`** — `com.acme.devices.site_checkout`, the verify-page
  query-kind scoped by `$caller_team_ids`.
- **`ui`** — `DevicesPanel` (`main` slot: the barcode → run → SSE → resume page)
  and `DevicesNav` (`sidebar-nav`).

---

## 9. How to test

### A. The Rust suite (no server needed)

```sh
# Domain + envelope round-trip
cargo test -p starter-setup-spi

# Run service, resume-from-cursor, HTTP e2e, extension import, authz
cargo test -p starter-setup --features "rest,mcp"

# Store impls (catalog overlay, run index, list_open semantics)
cargo test -p starter-store-sqlite --features "setup,flow,testing" --test setup

# P3a host token (binds text[], rejected in input, empty-array case)
cd nexus/backend && cargo test -p nexus-store --test query_bind
```

The headline test is `starter-setup/tests/run_and_resume.rs`: it launches a
template whose node fails the first time, asserts the run goes
`Failed + resumable + failed_node`, flips the node to succeed, resumes, and
asserts `Completed` — the full §8b loop, plus that trusted identity was seeded
from the principal (not the form).

### B. The demo extension against a live nexus-api

> The registry is **sealed at boot** — a newly-built bundle needs an API
> **restart** to surface. The in-repo copy under `NEXUS_EXTENSIONS_DIR`
> (`nexus/extensions`) is scanned at boot, so you only need `make build` + a
> restart.

```sh
# 1. Build the bundle: process binary (installed next to block.yaml) + the UI.
make -C nexus/extensions/com.acme.devices build

# 2. Start the stack (rebuilds nexus-api with the manifest + lint changes).
cd nexus && make dev          # API :4780, UI :4790

# 3. Full end-to-end probe (lifecycle + the live setup automation).
make -C nexus/extensions/com.acme.devices test
#   → list contains com.acme.devices
#   → detail resolves (state: validated, failure: null)
#   → ui/remoteEntry.js serves (200 + ETag, then 304)
#   → site_checkout query-kind registered (host-token guard enforced)
#   → caller added to the `hvac-ops` team (the template's allowed_teams)
#   → bundled add-device template present in the catalog
#   → POST .../run → 202 { run_id } → snapshot status=completed, progress 2/2
#   → PASS: com.acme.devices end to end
```

> The run step needs the caller in a team the template allows
> (`allowed_teams: [hvac-ops]`); `make test` creates that team + membership
> idempotently and re-logs in so the session principal carries it. Without it the
> run handler correctly returns `403 forbidden: principal shares no team with the
> template's allowed_teams` — the §10 team check, working as designed.

Manual probes against the running API:

```sh
# login (cookie session)
curl -fsS -c /tmp/nx.cookies -H 'content-type: application/json' \
  -d '{"email":"admin@nexus.local","password":"change-me-admin"}' \
  http://127.0.0.1:4780/auth/login

# extension loaded + healthy?
curl -fsS -b /tmp/nx.cookies http://127.0.0.1:4780/api/v1/extensions/com.acme.devices \
  | jq '{state, enabled, failure}'
# expect: {"state":"validated","enabled":"enabled","failure":null}

# UI bundle bytes
curl -fsSI -b /tmp/nx.cookies \
  http://127.0.0.1:4780/api/v1/extensions/com.acme.devices/ui/remoteEntry.js | grep -i etag
```

# Setup surface (now mounted under /api/v1):
curl -fsS -b /tmp/nx.cookies http://127.0.0.1:4780/api/v1/setup/templates | jq -c '.[].id'
# → "com.acme.add-device"
```

> The `/setup/*` routes are mounted in `nexus-api` (§7) — the device automation
> runs live, its steps executing in the extension child over the
> `ProcessNodeProxy` bridge. The in-crate `http_e2e.rs` remains the fast,
> Docker-free proof of the same flow.

### C. Restart safety note

`nexus/Makefile`'s `make dev` launches the API as a plain `cargo run` (no
cargo-watch). To restart just the API, kill it **by pid** — never by a broad
`pkill -f "cargo run"`, which matches every nexus-api instance (including a
second test instance on another port) and takes them all down.

---

## 10. Open questions (from the spec)

- **Q1 (resume policy).** Which `NodeError`s are fatal-halt vs. retryable-in-place;
  auto vs. manual vs. bounded-retry resume; auto-resume idempotent crashed runs on
  boot. Current default: all `NodeError` fatal-halt, manual resume, bounded
  auto-recovery on boot (`RunServiceConfig::max_auto_resume`).
- **Q6 (site == team?).** Recommended: a site **is** a `TeamRecord`, reusing the
  `$caller_team_ids` token and WS-13 nav — no new tenancy tier. Decide before the
  identity-scoped product pages ship.
- **Q9 (identity into nodes).** Server-seeded trusted slots (chosen — no engine
  change) vs. adding `Principal` to `NodeCtx`.
