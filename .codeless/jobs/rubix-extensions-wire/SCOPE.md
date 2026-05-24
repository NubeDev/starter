# Scope — rubix-extensions-wire

## Goal

Wire the **already-built** `starter-extensions` framework into rubix end-to-end. After this job, an operator can: (1) boot rubix-agent and see bundled extensions discovered and supervised, (2) call `POST /api/v1/extensions/<id>/{start,stop,restart,enable,disable}` via REST and observe the lifecycle change, (3) survive a restart with enabled-state preserved in PG, (4) install / uninstall an extension bundle by uploading or referencing a manifest, (5) load `test-ui-5` in a browser and see an extension-contributed React panel rendered into a named host slot via the existing `starter-ext-ui` Module-Federation runtime, served by rubix-agent's `GET /api/v1/extensions/<id>/ui/*` route.

This is a **wiring job, not an authoring one**. The `starter-extensions/` sibling workspace already ships every primitive we need:

- `starter-ext-spi` — the contract (Manifest, ExtensionBehavior trait, LifecycleState enum, JSON-RPC envelopes).
- `starter-ext-host` — the two-phase loader (`Loader::load_dir` + `ExtensionRegistry::seal`).
- `starter-ext-supervisor` — restart-with-backoff for the process-flavour extensions (Phase 2 deliverable, complete).
- `starter-ext-server` — the **admin router** (`router(admin) -> axum::Router`) covering `GET /extensions`, `GET /extensions/<id>`, lifecycle endpoints, event stream, UI bundle serving (`/extensions/<id>/ui/*`), and the `EnablementStore` trait seam.
- `starter-ext-sdk` + `starter-ext-sdk-macros` — author-side SDK (`#[derive(Extension)]`, `requires!{}`).
- `starter-ext-mcp` adapter — surfaces contributed tools over MCP (already what rubix's existing `FlowAsTool` complements).
- `starter-ext-ui` + `starter-ext-sdk-ts` — Module-Federation host runtime and the author-side singleton handshake.
- `rubix/extensions/com.rubix.example/` — example extension with `block.yaml` + process binary + skill + flow.

What's missing is **two integration points**:

1. **A PG-backed `EnablementStore` impl** — the `store.rs` doc explicitly calls this out: "v0.1 ships one concrete impl — `InMemoryEnablementStore` — so `TestApp` and the smoke tests work without a database; a real consumer plugs in a sqlx/sqlite/postgres-backed impl." Rubix is the first real consumer, so the PG impl lands **upstream** in a new `starter-ext-store-pg` crate per R2 — every future starter consumer benefits.
2. **The rubix-agent boot wiring** — construct `ExtensionHost` + `Supervisor` + `ExtensionAdmin` from `agent.toml`, mount `starter_ext_server::router(admin)` under `/api/v1/extensions`, expose the event stream on SSE, install the SDUI bundle-serving route.

Plus three smaller pieces: making `com.rubix.example` actually build (it's outside the workspace today), an integration test that drives the full lifecycle through REST, and a `test-ui-5` page that consumes the host-manager.

The success bar is: a fresh contributor reads `docs/design/extensions/README.md`, copies the example, runs `mani run demo`, sees their extension supervised, calls REST endpoints to enable/disable/restart, and renders its panel in the browser — without writing rubix-side glue. Every primitive is upstream; rubix just composes them.

## In scope

### Phase A — upstream: `starter-ext-store-pg` (the PG `EnablementStore` impl)

The contract crate makes this trivial. Single small crate.

- **`starter-extensions/crates/starter-ext-store-pg/`** — new crate.
  - `Cargo.toml` depending on `starter-ext-spi`, `starter-ext-server` (for the `EnablementStore` trait + `EnablementState` enum), `sqlx` (postgres + chrono features), `async-trait`, `serde`, `thiserror`.
  - `src/lib.rs` — barrel.
  - `src/store.rs` (~120 lines) — `PgEnablementStore { pool: PgPool }` impl `EnablementStore` for it. Methods: `get(id) -> Option<EnablementState>`, `set(id, state)`, `list_all() -> Vec<(ExtensionId, EnablementState)>`. Idempotent UPSERT on `(extension_id) DO UPDATE SET state = EXCLUDED.state, updated_at = NOW()`.
  - `src/migrations/0001_extensions_enablement.sql` — table `extensions_enablement(extension_id TEXT PRIMARY KEY, state TEXT NOT NULL CHECK (state IN ('enabled','disabled')), updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), updated_by TEXT NOT NULL)`. The `updated_by` column threads through from a `set_as(actor, id, state)` helper so audits work.
  - `tests/store_test.rs` — testcontainers PG, asserts roundtrip + UPSERT + list ordering.
- **`starter-extensions/Cargo.toml`** — add the new crate to `[workspace.members]`.
- **`starter-extensions/DOCS/extensions/scope/SCOPE.md`** — append a one-line decision under the existing "Decisions made — enable/disable persistence model" section naming `starter-ext-store-pg` as the PG impl.
- **No changes to `starter-ext-server`'s public API.** The trait already exists; we add an impl. If during implementation we discover the trait is awkward to impl over sqlx (e.g. lifetime issues with PgPool clones), raise an upstream issue and pause — do not patch the trait without confirming.

### Phase B — make `com.rubix.example` actually build

Today `rubix/extensions/com.rubix.example/process/Cargo.toml` exists but isn't part of any workspace. `cargo build` from the repo root fails per the warning we already see.

- **Decision: `rubix/extensions/` becomes its own workspace.** Extensions are deliberately decoupled from the rubix workspace (per SCOPE R8: extensions never depend on rubix-domain etc.). Adding it to the parent workspace would invite that dependency drift. The right move is `rubix/extensions/Cargo.toml` with its own `[workspace]` block that includes every `com.rubix.*/process/` member.
- **`rubix/extensions/Cargo.toml`** — new workspace manifest with `members = ["com.rubix.example/process"]`. Inherits Rust edition + lints from the parent via `workspace.package.edition.workspace = true` is **not** possible across workspaces; declare explicit edition/version per extension.
- **`com.rubix.example/process/Cargo.toml`** — confirm dependencies use the published `starter-ext-sdk = { version = "0.1", path = "../../../../starter-extensions/crates/starter-ext-sdk" }`. If the path dep is awkward across workspaces, a workspace-relative `path` works because both workspaces live under the same repo root.
- **`com.rubix.example/process/src/main.rs`** — keep as-is (it's already written to the sdk's API). Ensure `cargo build` produces `target/debug/rubix-example-extension` per the `block.yaml` `runtime.bin` field.
- **`rubix/extensions/README.md`** — present-tense doc: how to add a new extension, how to build it, how rubix-agent picks it up (link to the design doc rewritten in Phase E).
- **No change to `rubix/extensions/com.rubix.example/block.yaml`** beyond what's needed for the lifecycle to succeed (a `version: 0.2.0` bump only if the `block.yaml` schema changed between the doc-time spec and the spi today; otherwise leave version at 0.1.0).
- **CI** — root `.github/workflows/` learns to build the extensions workspace too (e.g. `cargo build --manifest-path rubix/extensions/Cargo.toml`). If a workflow doesn't already exist that's appropriate, add a minimal one in Phase E.

### Phase C — rubix-agent boot wiring

The integration core. Single verb file plus config + router wiring.

- **`rubix/crates/rubix-agent/src/boot/extensions.rs`** (~250 lines, verb file) — `pub async fn build_extension_admin(cfg: &AgentConfig, pg_pool: &PgPool) -> Result<ExtensionAdmin, BootError>`:
  1. Reads `cfg.extensions.{dir, autostart, enabled_by_default}` (a new `[extensions]` block in `agent.toml`).
  2. Constructs `PgEnablementStore::new(pg_pool.clone())` and runs its migration via sqlx.
  3. Constructs `ExtensionHost::new()` and calls `host.load_dir(&cfg.extensions.dir)` — the two-phase loader returns a list of `ExtensionRecord`s.
  4. Constructs `Supervisor::new(host.records())` for the process-flavour records.
  5. Constructs `ExtensionAdmin::new(host, supervisor, Arc::new(store))` per the public api.
  6. Calls `admin.autostart_enabled().await` so every extension that the PG store says is `Enabled` gets a live supervisor on boot.
  7. Returns the admin handle (cloneable; the router clones it into handlers).
- **`rubix/crates/rubix-agent/src/boot/config.rs`** — extend `AgentConfig` with:
  ```toml
  [extensions]
  enabled = true
  dir = "rubix/extensions"
  autostart_enabled_records = true
  ```
- **`rubix/dev/agent.toml`** — add the block under `[extensions]` with the dev defaults documented.
- **`rubix/crates/rubix-agent/src/main.rs`** — call `build_extension_admin` after the PG pool is constructed, before the router is assembled. Merge `starter_ext_server::router(admin)` under `/api/v1/extensions/*` into the rubix-agent router. Apply the existing `authz_gate` middleware — extension lifecycle is admin-only by default. The SSE event-stream route stays open under the same prefix.
- **MCP surface integration** — when an extension contributes tools (via `contributes.tools` in its block.yaml), they must auto-surface alongside rubix's bundled tools. `starter-ext-mcp` is the adapter that handles this; the wiring step is to pass the loaded extensions' tool contributions into the same `ToolRegistry` that `boot::mcp::build_mcp_surface` consumes. Phase A.3 of the goals-2-4-3 job extended `allowed_tools` for bundled flows; extension-contributed tools rely on a flow YAML referencing them explicitly (no implicit registration).
- **Audit + changelog** — every lifecycle action (`start`, `stop`, `restart`, `enable`, `disable`, `install`, `uninstall`) writes a `changelog` row through the existing `changelog_middleware` (kind = `extension.<action>`). This is automatic if the admin endpoints land under the same routing prefix that the middleware already gates.

### Phase D — install / uninstall + frontend wiring

The two pieces left after C: an install/uninstall surface on the backend and the test-ui-5 page that consumes the host.

- **Install / uninstall semantics.** A v1 honest answer:
  - `POST /api/v1/extensions/install` accepts a multipart upload of a tarball (block.yaml + process bin + ui assets) **OR** a JSON body with a registry URL to fetch from. The handler unpacks into `cfg.extensions.dir/<id>/`, validates the manifest, calls `ExtensionAdmin::reload_dir()` to pick it up. `Reversible` is **explicit-only** for installs — uninstall is the reverse, not an undo gate, because installs typically don't happen via the agent loop. Document this caveat in the design doc.
  - `DELETE /api/v1/extensions/<id>` calls supervisor stop, removes the directory, marks the PG row as `disabled` (we don't drop the row; future re-installs preserve audit history).
  - **Registry fetch is best-effort v1.** Default config has no registry URL; install-from-tarball is the operational path. A registry URL is added later when a real registry exists. Document this in the design doc.
- **`packages/test-ui-5/src/app/extensions/page.tsx`** (or wherever the routing lives in test-ui-5 today) — a page that:
  1. Constructs `ExtensionHostProvider` from `starter-ext-ui` with the rubix-agent base URL.
  2. Renders `<ExtensionSlot id="main">` somewhere visible.
  3. The example extension's `block.yaml` declares `contributes.ui.exposes.main: ./ui/main.tsx` (add this to the example's block.yaml in Phase B if missing).
  4. The host-manager fetches `GET /api/v1/extensions/com.rubix.example` to read the manifest, then loads `GET /api/v1/extensions/com.rubix.example/ui/main.tsx` (or the bundled MF artefact) and mounts it.
  - If the example's UI side isn't wired (it might not be — it's a process-flavour example that may not ship UI today), Phase D adds a minimal `ui/main.tsx` that renders "hello from com.rubix.example v0.1.0" plus the timestamp from a host-context call. This proves the round-trip.
- **`packages/test-ui-5/`** changes are confined to one new page and any host-provider wiring at the root layout. No starter-ui-kit changes.

### Phase E — integration tests + design doc + smoke + PR

- **`rubix/crates/rubix-agent/tests/extensions_lifecycle_test.rs`** — testcontainers PG; boots an agent with a fixture extension dir containing `com.rubix.example`; asserts: (1) `GET /api/v1/extensions` lists the example; (2) `POST .../start` transitions to `Running`; (3) `POST .../stop` to `Stopped`; (4) `POST .../restart` cycles cleanly; (5) `POST .../disable` then restart loses the supervisor; (6) `POST .../enable` brings it back; (7) PG row reflects the final state; (8) the event stream emits the expected lifecycle messages in order.
- **`rubix/docs/design/extensions/README.md`** — rewrite from the current scaffold-and-pointer style to present-tense:
  - The bootflow: `build_extension_admin` → host.load_dir → supervisor.autostart_enabled.
  - The REST surface mounted at `/api/v1/extensions/*` (link to `starter-ext-server`'s admin.rs docs).
  - The PG persistence (link to `starter-ext-store-pg`).
  - The 10-minute scaffold flow, rewritten so it actually works (today it cites a non-existent SDK; tomorrow it cites the real one).
  - The frontend wiring (one paragraph; link to `starter-ext-ui`).
  - The block.yaml shape (link to the SPI's manifest.rs docs).
  - Remove the "planned upstream — see STARTER-CHANGES.md" pointers; they're stale now that the framework exists.
- **`rubix/docs/sessions/<today>-extensions-wired.md`** — closing session note with the per-phase commit summary, the operator-runnable manual flow (boot → curl GET extensions → curl POST start → check supervisor state → curl event stream → see browser panel render), and the cargo+vitest+integration counts.
- **`rubix/docs/scope/THIN-SLICE.md`** — add an "Extensions wired" row to the "Goals lit up beyond the thin slice" table.
- **PR** — one PR off `codeless/rubix-extensions-wire` with phase-by-phase commits, reviewed in order.

## Out of scope

- **Authoring a non-example extension.** The example proves the end-to-end. New extensions (insights-iot, insights-energy, etc. that already exist in starter-extensions/crates/) are wireable but not in this job.
- **WASM runtime wiring.** `starter-ext-wasm` is built but not yet validated; this job covers the process flavour only (the example uses process). WASM gets a follow-up job once we have one working extension authored in WASM.
- **Builtin runtime wiring.** Same reason — process is the primary flavour for v1.
- **A registry server.** Install accepts a tarball upload or a registry URL, but no registry server exists or is in scope here. Operators upload bundles directly.
- **Per-tenant extension scoping.** Extensions today are host-global. Per-tenant enablement (different tenants see different sets) is a multi-tenant concern outside this job.
- **Permissions per lifecycle operation.** All lifecycle endpoints require admin role; finer-grained AuthZ (e.g. "this team can restart but not install") is a future job.
- **Extension contribution undo via `rubix.undo.last`.** Install/uninstall write changelog rows but do not register as `Reversible` per the goals-2-4-3 undo pattern. Reason: installs typically come from operators outside the agent loop. Document the caveat.
- **Hot reload of `block.yaml` without restart.** Possible via the loader's two-phase commit, but not in scope this job. Today's flow is: edit, restart.
- **Live LLM in CI.** Recorded fixtures remain the seam.
- **No `--no-verify`, no `--force` push.** No phasing markers in code.

## Constraints

- **R1 — One verb per file.** ≤ 400 lines hard, ~100 typical. `boot/extensions.rs` is allowed to reach ~250 because it composes 7 starter-extensions primitives; if it crosses 300, split into `boot/extensions/build.rs`, `boot/extensions/router.rs`, `boot/extensions/autostart.rs`.
- **R2 — Upstream-first.** Phase A lands `starter-ext-store-pg` in the `starter-extensions` workspace **first**, with its own crate + tests. Rubix consumes it in Phase C. R2 strictly.
- **R3 — Doc-tier rule.** Code comments link `docs/design/<area>/README.md` only. `./rubix/scripts/lint-doc-refs.sh` enforces it on every stage.
- **R4 — Tool outputs are `Diagnostic` + structured data.** Extension lifecycle endpoints return JSON (per starter-ext-server's existing shape), not Diagnostics — this is consistent with the admin-router contract.
- **R5 — Catalogue files.** Any new MessageKeys (e.g. `rubix.extension.started`, `rubix.extension.install_failed`) ship in both en.json + es.json same commit.
- **R6 — Tests live with the code in the same commit.**
- **R8 — Extensions never depend on rubix-*.** The example extension uses `starter-ext-sdk` only — verify in Phase B that no `rubix-*` path-deps creep in.
- **Commit messages.** `feat(starter-ext-store-pg):` for Phase A, `chore(rubix-extensions-workspace):` for Phase B, `feat(rubix-agent):` for Phase C, `feat(rubix-agent+test-ui-5):` for Phase D, `docs+test:` for Phase E.

## Open questions

1. **Where does `cfg.extensions.dir` default?** Default to `rubix/extensions` in dev (relative to repo root); for prod, `/var/lib/rubix/extensions`. Confirm the prod path with operator before E.
2. **Does `starter-ext-server`'s admin router include the SSE event-stream endpoint, or is that a separate router function?** Phase C.1 must confirm by grep; if separate, Phase C mounts both.
3. **Does the example extension already ship UI assets?** Phase B confirms; if not, Phase D adds a minimal `ui/main.tsx` so the frontend round-trip can be tested.
4. **Should the example extension's flow YAML auto-surface as an MCP tool?** Per SCOPE R7 yes, via `starter-ext-mcp`. Phase C.2 verifies `tools/list` includes `com.rubix.example.echo` (the example's contributed tool) post-wiring.
5. **`changelog` actor for autostart-on-boot.** Bootstrap actor (`op@example.com` or a synthetic `system` principal). Default: a `system` principal so the changelog clearly distinguishes operator actions from boot-time autostart. Confirm at C.3.
6. **What happens when the PG migration for `extensions_enablement` collides with an existing rubix migration on schema version numbers?** Phase A.2 confirms the migration uses the starter-ext-store-pg crate's own migration root, not rubix's; the two migration sets are independent.

## References

- `DOCS/extensions/scope/SCOPE.md` — the authoritative starter-extensions scope (1483 lines). This job consumes what's in Phases 1+2+3+5; Phase 4 (wasm) and Phases 6-8 are out of scope.
- `DOCS/extensions/scope/FLOW-NODES.md` — how flow nodes flow through extensions; relevant when an extension contributes a `NodeBehavior` (not in this job's scope but referenced).
- `starter-extensions/crates/starter-ext-spi/` — the contract.
- `starter-extensions/crates/starter-ext-host/` — the loader.
- `starter-extensions/crates/starter-ext-supervisor/` — the lifecycle manager.
- `starter-extensions/crates/starter-ext-server/` — the admin router + bundle serving.
- `starter-extensions/crates/starter-ext-server/src/store.rs` — the `EnablementStore` trait Phase A implements.
- `starter-extensions/packages/starter-ext-ui/` — the frontend host-manager.
- `starter-extensions/packages/starter-ext-sdk-ts/` — the author-side TS SDK.
- `rubix/extensions/com.rubix.example/` — the worked example this job makes live.
- `rubix/docs/design/extensions/README.md` — gets rewritten in Phase E.
- `rubix/SCOPE.md` R7, R8 — the rules this job lives under.
- `rubix/docs/sessions/2026-05-24-goals-2-4-3-landed.md` — the verification-evidence shape Phase E mirrors.
