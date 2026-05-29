# Scope — comprehensive-extension-management

The authoritative design lives at
[/home/user/code/rust/starter/rubix/docs/scope/extensions/comprehensive-extension-management.md](/home/user/code/rust/starter/rubix/docs/scope/extensions/comprehensive-extension-management.md).
This brief is the trimmed per-job scope. Where this disagrees with the
design doc, **the design doc wins** — fix this file rather than diverge.
The scope is **locked**: the design doc's "Locked decisions" section
resolves every question. Do not reopen them.

## Goal

Turn the rubix `/extensions` admin surface from "list + enable/disable"
into a comprehensive management surface. After this job an operator can,
for any extension:

1. **See its issues** — one consolidated, severity-ranked list folding
   manifest-validation failures, crashes, restart-cap exhaustion, health
   timeouts, capability violations, and worker failures.
2. **See its PID + process stats** — live pid, uptime, RSS, CPU,
   restart count (process-flavour only; builtin/wasm report null).
3. **See its metrics** — sampled counters across tool calls, REST
   requests, worker runs, capability violations, ring evictions.
4. **Upload / delete** — install already exists (next-boot); delete now
   offers a complete data cleanup.
5. **Clean up its data** — uninstall with `?purge=true` removes the
   extension's warehouse tables, its enablement row, its UI/**sidebar**
   cache, and its registered skills, with a dry-run preview first.

Hard architectural rule: **as much logic as possible lives in the
reusable `starter-ext-*` crates; rubix only wires + projects.** New
types, the metrics registry, the cleanup trait + built-in providers, and
every HTTP handler live in starter. Rubix supplies only the two cleanup
providers that need warehouse/skill knowledge and the response
projection + UI.

## In scope (five stages + one REVIEW gate, mapping to the design's P1–P5)

- **Stage 1 — Issues.** `ExtensionIssue` in `starter-ext-spi`;
  `SupervisorHandle::issues()`; `ExtensionRecord::issues()`;
  `GET /extensions/{id}/issues`.
- **Stage 2 — PID + process stats.** `ProcessStats` + flavour
  discriminator; live pid cell on the handle; `/proc` sampler on the
  health tick; `GET /extensions/{id}/process`.
- **Stage 3 — Metrics.** New leaf crate `starter-ext-metrics`;
  `MetricsRegistry`; counter bumps in mcp / server-REST / workers;
  `GET /extensions/{id}/metrics`.
- **REVIEW gate** — before the destructive cleanup lands.
- **Stage 4 — Cleanup.** `CleanupProvider` trait + `CleanupItem`;
  built-in `EnablementRow` / `UiCache` / `I18nCache` providers;
  `DELETE /extensions/{id}?purge=true` + `GET /extensions/{id}/cleanup`
  dry-run; `restart_required` on the list projection.
- **Stage 5 — Rubix wiring + frontend.** Register
  `WarehouseCleanupProvider` + `SkillCleanupProvider`; project envelopes;
  add Issues / Process / Metrics tabs + an Uninstall dialog with the
  dry-run preview; add the client hooks.

## Out of scope

- **Hot-mount of newly installed extensions without restart.** The
  `ExtensionRegistry` is sealed by design; install stays next-boot. We
  only surface `restart_required` — no registry-mutation hack.
- **Cross-extension / fleet-wide aggregate dashboards.** Per-extension
  views only.
- **Resource enforcement** (cgroup limits, OOM-kill policy). We surface
  RSS/CPU as gauges; we do not cap them.
- **Registry-URL pull install.** Stays `501` as today.
- **A second metrics tick / push-based collector.** Sampling rides the
  existing health tick only (locked decision #2).
- **Reporting a host pid for builtin/wasm extensions.** They report
  `null` (locked decision #3).
- **Touching the existing enable/disable/restart/events/install paths**
  beyond the additive `restart_required` flag and the enablement-row
  delete in cleanup. The current behaviour is preserved; new endpoints
  are additive.

## Constraints

- **Starter-first.** Any logic that does not require warehouse or skill
  knowledge lives in `starter-ext-*`, not rubix. Rubix gets the two
  warehouse/skill cleanup providers, the envelope projection, and the
  frontend — nothing more. A reviewer should be able to point at each new
  line and say which crate owns it and why it could not live upstream.
- **No English in the API.** Issues and lifecycle responses carry a
  stable `code` string; rubix maps to the `rubix.extension.*` MessageKey
  namespace. No human-readable strings baked into starter responses.
- **Cleanup is namespace-scoped, dry-run-able, audited.** A provider may
  only remove data the extension owned: `com_<id>__*` tables, its own
  enablement row, its own UI/i18n cache keys, its own skills. `discover`
  before `purge`; every destructive step logs the caller principal.
- **`purge` is idempotent** — no-op `200`, never `404` (locked
  decision #4).
- **No new always-on thread per extension.** The `/proc` sampler runs on
  the existing health wakeup (locked decision #2).
- **Read paths degrade gracefully without a live supervisor.** Issues,
  last-known state, and `process: null` all return from the registry +
  store for builtin / wasm / disabled extensions.
- **R1** — keep files within the repo's line limit; new modules
  (`issue.rs`, `process.rs`, `metrics.rs`, `cleanup.rs`, `issues.rs`) are
  split rather than bloated.
- **No `--force`, no `--no-verify`.** If a hook fails, fix the cause.

## Deliverables (what "done" looks like)

1. `codeless/comprehensive-extension-management` branch with one commit
   per stage (five code stages = five commits; the REVIEW gate commits
   the stage that led to it and pauses the next).
2. New endpoints live and tested: `GET /extensions/{id}/issues`,
   `/process`, `/metrics`, `/cleanup`; `DELETE /extensions/{id}?purge=`.
3. New crate `starter-ext-metrics` added to the starter-extensions
   workspace and consumed by the mcp / server / workers adapters.
4. Uninstall with `?purge=true` removes warehouse tables, the enablement
   row (not a `disabled` ghost), the UI/sidebar cache, and skills; the
   dry-run preview lists each item before deletion.
5. `cargo build` + `cargo clippy --all-features -- -D warnings` +
   `cargo fmt --check` green for the `starter-extensions` workspace at
   every starter stage boundary, and for the `rubix` workspace at the
   stage-5 boundary.
6. Frontend `tsc -b` + `vite build` + lint green at the stage-5
   boundary; the detail page shows Issues / Process / Metrics tabs and an
   Uninstall dialog that previews the purge manifest before confirming.
7. Builtin/wasm extensions show no Process tab and `process: null`;
   process-flavour extensions show a live pid that clears on exit.

## Open questions — RESOLVED (locked in the design doc, 2026-05-29)

All four are settled in the design doc's "Locked decisions" section; the
job must not reopen them. Summarised here for the per-stage prompt:

1. **Metrics registry placement** → new leaf crate `starter-ext-metrics`
   (adapters → metrics ← supervisor; no adapter depends on the
   supervisor just to bump a counter).
2. **Sampling cadence** → ride the existing health tick (default 5s); no
   second timer; no sampler for flavours with no health loop.
3. **Builtin/wasm process stats** → `null` + a `flavour` discriminator;
   `/process` returns `404 ext.process.not_running`; UI hides the tab.
   Never report a host pid for builtin.
4. **`purge` idempotency** → no-op `200 cleanup.succeeded` with the items
   actually removed; never `404`.

## References

- Design doc (authoritative):
  [/home/user/code/rust/starter/rubix/docs/scope/extensions/comprehensive-extension-management.md](/home/user/code/rust/starter/rubix/docs/scope/extensions/comprehensive-extension-management.md).
- Sibling scope doc for the warehouse data path the cleanup provider
  must respect:
  [/home/user/code/rust/starter/rubix/docs/scope/extensions/extension-data-to-dashboard.md](/home/user/code/rust/starter/rubix/docs/scope/extensions/extension-data-to-dashboard.md).
- Current lifecycle handlers being extended:
  `starter-extensions/crates/starter-ext-server/src/lifecycle.rs`.
- Current event ring (source of issues + the captured pid):
  `starter-extensions/crates/starter-ext-supervisor/src/event_ring.rs`.
- Rubix boot wiring point for providers:
  `rubix/crates/rubix-agent/src/boot/extensions.rs`.
