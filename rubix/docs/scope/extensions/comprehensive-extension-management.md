# Comprehensive extension management (long-term plan)

> **Tier:** plan, not system-as-it-is. Lives in `docs/scope/` per
> [HOW-TO-CODE.md §0a](../../HOW-TO-CODE.md). Source code must not
> reference this file — once a layer below lands, its design moves
> into `docs/design/extensions/README.md` and code links there.

**Status:** proposal
**Author:** rubix-agent
**Date:** 2026-05-29

## Problem

The `/extensions` admin surface today lists extensions and toggles
enable / disable / reload-UI. That is enough to demo, not enough to
*operate*. When an operator runs a fleet of extensions they need to
answer five questions the current surface cannot:

1. **"What's wrong with this extension?"** — crashes, capability
   violations, worker errors, manifest-validation failures, and health
   timeouts are all recorded, but scattered across the event ring,
   the registry record, and per-worker state. There is no single
   *issues* view.
2. **"What's its PID?"** — the supervisor captures the child OS pid in
   an `EventKind::Spawned { pid }` ring entry
   ([`event_ring.rs:38-43`](../../../starter-extensions/crates/starter-ext-supervisor/src/event_ring.rs#L38-L43))
   but there is no live accessor on `SupervisorHandle` and no endpoint —
   an operator has to scrape the event log to find a pid that may be stale.
3. **"Install / remove a bundle."** — `POST /extensions/install` and
   `DELETE /extensions/{id}` exist
   ([`lifecycle.rs`](../../../starter-extensions/crates/starter-ext-server/src/lifecycle.rs))
   but install only surfaces on the *next boot* and delete is incomplete
   (see #4).
4. **"Clean up everything it left behind."** — uninstall removes the
   bundle directory and writes `EnablementState::Disabled`, but **leaks**
   the extension's warehouse tables (`com_<id>__*`), its enablement row
   (left as a `disabled` ghost rather than deleted), its UI bundle cache,
   its registered skills, and its event-bus subscriptions. "Clean up its
   data, e.g. sidebar" is the headline gap.
5. **"How is it doing?"** — the only metrics exposed are `restart_count`
   and `capability_violations`. There is no uptime, memory/CPU sample,
   request/dispatch counters, or per-transport (tool / REST / worker)
   throughput.

This plan makes extension management **comprehensive** across those five
areas, with a hard architectural constraint:

> **As much logic as possible lives in the `starter-ext-*` crates
> (reusable by any starter consumer), and rubix only wires + projects.**

That mirrors the existing split — `starter-ext-server` owns the HTTP
handlers, `starter-ext-supervisor` owns process lifecycle, and
[`rubix-agent`](../../../rubix/crates/rubix-agent) only composes them
([`compose.rs:68-71`](../../../rubix/crates/rubix-agent/src/bin/rubix_agent/compose.rs#L68-L71)).

## Principles

1. **Starter owns the mechanism; rubix owns the policy + projection.**
   New data structures (`ExtensionIssue`, `ProcessStats`, `ExtensionMetrics`,
   the cleanup trait) and the HTTP handlers that serve them live in
   `starter-ext-*`. Rubix supplies the concrete *cleanup providers*
   (warehouse drop, skill unregister) and projects responses into the
   admin-console envelope.
2. **Every diagnostic has a stable code.** Issues, like the existing
   lifecycle responses, carry a stable `code` string the consumer maps to
   its own `MessageKey` catalog (rubix uses `rubix.extension.*`). No
   English in the API.
3. **Cleanup is a capability-scoped, auditable, declared operation.** An
   extension can only have data removed in the namespaces it owned
   (`com_<id>__*` tables, its own enablement row, its UI/i18n cache keys).
   Cleanup is dry-run-able and every destructive step is logged with the
   caller principal.
4. **Metrics are sampled, not pushed.** The supervisor already runs a
   health loop; metrics piggyback on it. No new always-on collector
   thread per extension.
5. **Read paths never need a running supervisor.** Issues, PID (last
   known), and metrics degrade gracefully for builtin/wasm/disabled
   extensions — they return what the registry + store know without
   requiring a live process.

## What exists today (audit)

| Capability | Where | State |
|---|---|---|
| List / detail | [`routes.rs:43-153`](../../../starter-extensions/crates/starter-ext-server/src/routes.rs#L43-L153) | ✅ |
| Enable / disable / restart | [`routes.rs:168-314`](../../../starter-extensions/crates/starter-ext-server/src/routes.rs#L168-L314) | ✅ |
| Event ring (state, spawn+pid, crash, stderr, health, cap-violation) | [`event_ring.rs`](../../../starter-extensions/crates/starter-ext-supervisor/src/event_ring.rs) | ✅ |
| Events snapshot + SSE | [`events.rs:61-104`](../../../starter-extensions/crates/starter-ext-server/src/events.rs#L61-L104) | ✅ |
| Install (multipart tarball) | [`lifecycle.rs:61-234`](../../../starter-extensions/crates/starter-ext-server/src/lifecycle.rs#L61-L234) | ⚠️ next-boot only |
| Uninstall | [`lifecycle.rs:240-296`](../../../starter-extensions/crates/starter-ext-server/src/lifecycle.rs#L240-L296) | ⚠️ leaks data |
| `restart_count`, `capability_violations` | list/detail projection | ⚠️ thin |
| Issues view | — | ❌ |
| Live PID accessor | — | ❌ |
| Data cleanup (warehouse / skills / cache / row) | — | ❌ |
| Rich metrics (uptime / mem / cpu / dispatch counts) | — | ❌ |

## Design

The five features map to changes across three starter crates plus a thin
rubix wiring layer. The new public API on the admin router:

```
GET    /extensions/{id}/issues        consolidated diagnostics
GET    /extensions/{id}/process       live pid + process stats
GET    /extensions/{id}/metrics       sampled counters + gauges
DELETE /extensions/{id}?purge=true    uninstall + full data cleanup
GET    /extensions/{id}/cleanup       dry-run: what purge would remove
```

### 1. Issues — `starter-ext-supervisor` + `starter-ext-server`

A single read model that folds every known failure source into one
ordered list. **New in `starter-ext-spi`** (the contract crate so every
adapter can produce them):

```rust
// starter-ext-spi/src/issue.rs  (new)
pub struct ExtensionIssue {
    pub code: IssueCode,          // stable enum, serializes to "ext.issue.*"
    pub severity: Severity,       // Info | Warning | Error | Fatal
    pub at: SystemTime,
    pub detail: String,           // operator-facing context, not localized key
    pub source: IssueSource,      // Manifest | Supervisor | Worker | Capability | Health
    pub seq: Option<u64>,         // ring seq when derived from an event
}

pub enum IssueCode {
    ManifestInvalid, NamespaceViolation, CapabilityMismatch, // from ExtensionRecord.failure
    Crashed, RestartCapExceeded, HealthTimeout,              // from event ring
    CapabilityViolation,                                     // from counter + Stderr-classified
    WorkerFailed,                                            // from WorkerState.last_error
}
```

**`starter-ext-supervisor`** gains `SupervisorHandle::issues() -> Vec<ExtensionIssue>`
that derives issues from the event ring + violation counter + worker
states (it already owns all three). **`starter-ext-host`** gains
`ExtensionRecord::issues()` returning the manifest-validation failure (if
state is `Failed`) as a single `Fatal` issue — this is the path that
works with **no live supervisor**.

**`starter-ext-server`** adds the `GET /extensions/{id}/issues` handler in
a new `issues.rs` module: merges `record.issues()` with
`handle.issues()` (when a handle exists), sorts by `at` desc, supports
`?severity=error&since=<seq>` filters. Pure projection — no rubix code.

### 2. PID + process stats — `starter-ext-supervisor` + `starter-ext-server`

Today the pid is only in a historical ring entry. We add live state:

```rust
// starter-ext-supervisor: store the current child pid alongside the
// watch::Receiver<LifecycleState> already held in SupervisorHandle.
impl SupervisorHandle {
    pub fn pid(&self) -> Option<u32>;          // None when not Running
    pub fn process_stats(&self) -> Option<ProcessStats>;
}

// starter-ext-spi/src/process.rs (new) — flavour-agnostic shape
pub struct ProcessStats {
    pub pid: u32,
    pub started_at: SystemTime,
    pub uptime: Duration,
    pub rss_bytes: Option<u64>,    // sampled, platform best-effort
    pub cpu_pct: Option<f32>,      // sampled over the health interval
    pub restarts: u64,
}
```

The pid is stored in a `watch`/`ArcSwap` cell updated next to the existing
`EventKind::Spawned { pid }` push at
[`supervisor.rs:602-603`](../../../starter-extensions/crates/starter-ext-supervisor/src/supervisor.rs#L602-L603),
and cleared on exit. RSS/CPU sampling is done **on the existing health
tick** (no new thread) by reading `/proc/<pid>/stat` + `/statm` on Linux
(the platform per the environment); other platforms return `None`. This
keeps the sampler dependency-free and inside starter.

`GET /extensions/{id}/process` in `starter-ext-server` returns
`ProcessStats` or `404 ext.process.not_running` for builtin/wasm/stopped.

### 3. Metrics — `starter-ext-supervisor` (+ adapter counters)

`ExtensionMetrics` aggregates what the supervisor and the transport
adapters already see. The adapters (`starter-ext-mcp`, `-server` REST,
`-workers`) increment shared atomic counters keyed by extension id; the
supervisor exposes the process gauges:

```rust
// starter-ext-spi/src/metrics.rs (new)
pub struct ExtensionMetrics {
    pub process: Option<ProcessStats>,        // reuses §2
    pub lifecycle_state: LifecycleState,
    pub restarts_total: u64,
    pub capability_violations_total: u64,
    pub tool_calls_total: u64,                // from starter-ext-mcp
    pub tool_errors_total: u64,
    pub rest_requests_total: u64,             // from starter-ext-server rest dispatch
    pub worker_runs_total: u64,               // from starter-ext-workers
    pub worker_failures_total: u64,
    pub events_dropped_total: u64,            // ring evictions (already monotone seq)
}
```

A small `MetricsRegistry` holds `DashMap<ExtensionId, Counters>` with
atomic increments — adapters get a cheap `&MetricsRegistry` handle at
wiring time. `GET /extensions/{id}/metrics` serves the merged view.

> **Decision (resolved):** the registry lives in a new leaf crate
> `starter-ext-metrics`, not in `starter-ext-supervisor`. The tool / REST /
> worker adapters already exist as independent crates and only need to bump
> a counter; making them depend on the supervisor (process spawning,
> signals, `/proc` sampling) for that is both heavy and circular — the
> supervisor itself reads those counters to build `/metrics`. A leaf crate
> keeps every dependency arrow pointing one way: adapters → metrics ←
> supervisor.

### 4. Data cleanup ("e.g. sidebar") — trait in starter, providers in rubix

This is the crux of the starter/rubix split. The **mechanism** —
discovering what an extension owns and orchestrating removal in a
dry-run-able, audited way — lives in starter. The **knowledge of how to
drop a Timescale table or unregister a skill** lives in rubix, because
only rubix owns the warehouse and skill registry.

```rust
// starter-ext-server/src/cleanup.rs (new)
/// One reclaimable resource an extension owns.
pub struct CleanupItem {
    pub kind: CleanupKind,     // WarehouseTable | EnablementRow | UiCache | I18nCache | Skill | Subscription
    pub label: String,         // e.g. "com_rubix_geo__pins"
    pub bytes: Option<u64>,    // best-effort size for the dry-run report
}

/// Implemented per-consumer; rubix provides the warehouse + skill ones,
/// starter provides the built-in cache + enablement-row ones.
#[async_trait]
pub trait CleanupProvider: Send + Sync {
    async fn discover(&self, id: &ExtensionId, m: &Manifest) -> Vec<CleanupItem>;
    async fn purge(&self, id: &ExtensionId, items: &[CleanupItem]) -> Result<(), CleanupError>;
}
```

Built-in providers (in `starter-ext-server`, no rubix needed):

- **EnablementRowProvider** — `DELETE FROM extensions_enablement WHERE
  extension_id = $1` (today's uninstall only flips to `disabled`, leaving
  a ghost row; see
  [`lifecycle.rs:260`](../../../starter-extensions/crates/starter-ext-server/src/lifecycle.rs#L260)).
- **UiCacheProvider / I18nCacheProvider** — evict the ETag/byte caches in
  [`ui.rs`](../../../starter-extensions/crates/starter-ext-server/src/ui.rs)
  for the extension's path prefix. **This is the literal "sidebar"
  cleanup** — the `sidebar` / `sidebar-nav` Module-Federation slots are
  served from this cache, so an uninstalled extension must drop them or
  its panel lingers until restart.

Rubix-supplied providers (registered at compose time):

- **WarehouseCleanupProvider** (`rubix-agent`) — lists + drops
  `com_<id>__*` tables and their continuous aggregates, scoped strictly to
  the extension's namespace (mirrors the DDL path in
  [`extension_tables.rs`](../../../rubix/crates/rubix-agent/src/boot/extension_tables.rs)).
- **SkillCleanupProvider** (`rubix-agent`) — `SkillRegistry::remove` for
  skills contributed by the bundle.

`DELETE /extensions/{id}?purge=true` runs uninstall then every provider's
`purge`; `?purge=false` (default) keeps today's behaviour.
`GET /extensions/{id}/cleanup` runs only `discover` and returns the
`Vec<CleanupItem>` as a dry-run manifest so the operator sees exactly what
will be dropped (and total bytes) before confirming. Every purge step logs
`target: "starter_ext_server::cleanup"` with the caller principal.

### 5. Upload made live-ish — keep next-boot, surface clearly

Hot-mount-after-seal remains out of scope (the `ExtensionRegistry` is
sealed by design — [`registry.rs`](../../../starter-extensions/crates/starter-ext-host/src/registry.rs)).
The install response already says "surfaces on next boot"; we make that
explicit in the API by returning the `pending_restart: true` flag and
exposing a `restart_required` field on the list projection so the UI can
badge freshly-installed extensions. No registry-mutation hack.

## Rubix integration (the thin layer)

1. **Register cleanup providers** in
   [`boot/extensions.rs`](../../../rubix/crates/rubix-agent/src/boot/extensions.rs)
   on the `ExtensionAdminBuilder`: `.with_cleanup_provider(warehouse)`
   `.with_cleanup_provider(skills)`. Built-in providers auto-register.
2. **Project new responses** into the admin-console envelope in
   [`admin/extensions.rs`](../../../rubix/crates/rubix-agent/src/admin/extensions.rs)
   — map `IssueCode` → `rubix.extension.issue.*` MessageKeys, expose
   `issues`, `process`, `metrics` on the detail view.
3. **Frontend** (`frontend/src/routes/extensions.$extId.$.tsx`): add
   tabs — *Issues* (severity-coloured list), *Process* (pid, uptime,
   mem/cpu), *Metrics* (counters), and a *Uninstall* dialog that calls the
   `cleanup` dry-run first and shows the purge manifest before confirming.
   New hooks in `packages/rubix-client-react/src/hooks/extensions.ts`:
   `useExtensionIssues`, `useExtensionProcess`, `useExtensionMetrics`,
   `useExtensionCleanupPreview`, `useExtensionPurge`.

## Phasing

Each phase is a landable PR; starter changes precede their rubix wiring.

| Phase | Crate(s) | Deliverable |
|---|---|---|
| P1 | spi, supervisor, host, server | `ExtensionIssue` + `/issues` endpoint + record-level issues |
| P2 | spi, supervisor, server | `pid()` accessor, `ProcessStats`, `/process` endpoint, `/proc` sampler |
| P3 | metrics, mcp, server, workers | counter registry + `/metrics` endpoint |
| P4 | server (+ rubix providers) | `CleanupProvider` trait, built-in providers, `?purge` + `/cleanup` dry-run |
| P5 | rubix-agent, frontend | wire providers, project envelopes, UI tabs + uninstall dialog |

## Out of scope

- Hot-mount of newly installed extensions without restart (sealed
  registry is a deliberate invariant).
- Cross-extension / fleet-wide aggregate dashboards (per-extension only).
- Resource *enforcement* (cgroup limits, OOM kill policy) — we surface
  RSS/CPU, we don't cap them here.
- Registry-URL pull install (still 501, per
  [`lifecycle.rs:82-88`](../../../starter-extensions/crates/starter-ext-server/src/lifecycle.rs#L82-L88)).

## Locked decisions

The scope is locked — there are no open questions. The following were
resolved before the implementation job was submitted; the design above
reflects them.

1. **Metrics counter registry → new leaf crate `starter-ext-metrics`.**
   See §3. Adapters (mcp / server / workers) depend on the leaf crate, not
   on the supervisor; the supervisor reads the same registry to build
   `/metrics`. Dependency arrows point one way: adapters → metrics ←
   supervisor.
2. **CPU/RSS sampling rides the existing health tick.** No second timer.
   The supervisor already wakes on `supervision.health.interval_ms`
   (default 5s); the `/proc/<pid>` sampler runs in that same wakeup while
   the state is `Running`. For flavours/extensions with no health loop
   (builtin/wasm), there is no sampler and `process` is `null` (see #3).
   5s granularity is accepted as adequate for an operator gauge; a faster
   cadence would be a later, opt-in change behind a config key, not part
   of this job.
3. **Process stats are process-flavour only; builtin/wasm report `null`.**
   The response carries a `flavour` discriminator (`builtin` | `wasm` |
   `process`). For non-process flavours `process`/`pid` are `null` and the
   `GET /process` endpoint returns `404 ext.process.not_running`; the UI
   hides the Process tab. The host pid is never reported for builtin
   extensions — it is meaningless (shared with every other builtin) and
   misleading.
4. **`purge` is idempotent — a no-op `200`, never `404`.** Running purge
   against an already-uninstalled or partially-cleaned id removes whatever
   leftovers remain (e.g. a ghost enablement row) and returns
   `200 cleanup.succeeded` with the items actually removed (possibly
   empty). This lets the UI always offer "clean up leftovers" for a ghost
   row without special-casing the not-found path.
