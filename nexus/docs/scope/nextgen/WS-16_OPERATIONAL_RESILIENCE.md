# WS-16 — Operational Resilience: liveness canary, task watchdog, supervisor

> **Status:** Not started · **Wave:** 2 (production hardening — slots beside WS-09) · **Owner:** _unassigned_
> **Depends on:** nothing new. Pure additive boot wiring in `nexus-api`. No schema, no migration, no DTO.
> **Source of the idea:** ported from the sibling `rubix` workspace, which earned these the hard way
> (a tokio runtime wedge that took hours to diagnose by absence-of-log archaeology). See
> `rubix/crates/rubix-agent/src/boot/{task_watchdog,runtime_canary}.rs` and
> `rubix/crates/rubix-watchdog/`.
> **Read first:** `WS-09_PRODUCTION_HARDENING.md` (this is its operational sibling), and the rubix
> source files cited inline below.
> **Verified:** nexus code claims below grepped on 2026-06-11 — re-grep file:line before building.

## Goal

Nexus has a strong data plane but a thin **operational floor**. Today the server can stop responding
in three different ways that all look identical from outside — "the agent stopped answering" — and
nothing in the process distinguishes them or even notices the death of a background task. This WS adds
the three pieces of resilience infra rubix already proved out, smallest-first:

1. **Runtime liveness canary** + a real `/livez` `/readyz` split (so a wedge is *visible*).
2. **Task watchdog** wrapping every long-lived `JoinHandle` (so a dead background task is *loud*).
3. **Watchdog supervisor binary** (an OS-peer process that probes `/livez` and escalates) — the
   defense that can fire even when the runtime itself is wedged.

None of this changes product behaviour. It changes how fast an incident is diagnosed: the difference
between a 5-minute and a 5-hour outage.

## Current state (evidence) — what nexus already has, and the gap

- ✅ `/health`, `/metrics`, `/openapi.json` — added by `starter_server::ServerBuilder`
  (`crates/nexus-api/src/serve.rs:4`). **But it is a single endpoint** — it cannot tell a runtime
  wedge from an HTTP-layer wedge from a slow handler.
- ✅ Long-lived background tasks already exist and are spawned with the bare-spawn pattern:
  - alert scheduler — `nexus_api::alerting::schedule::spawn(state)` (`crates/nexus-api/src/main.rs:136`)
  - detection scheduler (WS-15) — `nexus_api::detecting::schedule::spawn(state)` (`main.rs:141`)
  - changelog prune — `nexus_api::changelog::prune::spawn(...)` (`main.rs:145`)
  - each does `tokio::spawn(async move { loop { ticker.tick().await; ... } })`
    (e.g. `crates/nexus-api/src/alerting/schedule.rs:25`).
  - **The gap:** if any of these panics, returns early, or is aborted, **nobody notices.** You infer
    the death from absence — "the last 'evaluated due rules' log line is older than the tick interval"
    — exactly the archaeology this WS eliminates.
- ✅ Alert notify already has exponential backoff with jitter (`crates/nexus-api/src/alerting/notify/retry.rs:36`)
  — good prior art; the canary tick loop should follow the same `tokio::time` injection style for
  testability.
- ❌ **No runtime canary.** Nothing proves the tokio runtime is advancing.
- ❌ **No task watchdog.** Bare `tokio::spawn`; handles leak into the runtime via the
  `nexus_api::…::spawn(state)` calls with no death detection.
- ❌ **No external supervisor.** A wedge inside the runtime cannot be detected by anything living
  inside that same runtime.

> **Explicitly out of scope / already done** — do not rebuild these:
> - **Undo / Reversible** is *already shipped* (WS-12): `crates/nexus-api/src/reversible/` has 7 kinds
>   (dashboard, panel, folder, nav_node, datasource, query_kind, manifest) and undo routes at
>   `routes/undo/`. Rubix's `ReversibleTool` adapter is the *same idea* nexus already has — nothing to port.
> - **Migration-source isolation** is *already done*: `crates/nexus-api/src/bootstrap.rs` runs 4
>   namespaced `MigrationSource`s (`auth_users`, `authz`, `nexus`, `ext_store`), each with its own
>   `_sqlx_migrations_<name>` ledger. Rubix's pattern is already nexus's pattern.

## Phase A — Runtime canary + `/livez` `/readyz` (smallest, do first)

**Port:** `rubix/crates/rubix-agent/src/boot/runtime_canary.rs` (~123 lines) almost verbatim.

The mechanism: a `Canary` holding an `Arc<AtomicU64>`, bumped to `now_unix_secs` once per second by a
spawned tick task. A handler reads `canary.staleness()`; if it exceeds the staleness budget (rubix
uses 5s — generous, so a transient stop-the-world doesn't trip it), `/livez` returns 503.

The three failure modes this disambiguates (rubix's doc comment, worth keeping):

1. **Runtime wedge** — every tokio worker parked on a futex; the atomic stops advancing. `/livez` → 503.
   Cause is runtime-internals territory: a sync `Mutex` held across `.await`, a blocking subscriber, etc.
2. **HTTP-layer wedge** — runtime alive (atomic advancing) but the axum accept loop or a tower layer is
   stuck. `/livez` → 200 yet external probes time out → operator looks at middleware.
3. **App-layer wedge** — both liveness endpoints fine; one slow handler. Standard handler debugging.

### Wiring into nexus

- New module `crates/nexus-api/src/boot/runtime_canary.rs` (create the `boot/` module — nexus doesn't
  have one yet; `main.rs` does its boot inline today).
- `let (canary, _tick) = runtime_canary::spawn();` in `main.rs`, store `canary` in `AppState`
  (`crates/nexus-api/src/state.rs`) so the route can read it.
- Add `/livez` (reads `canary.staleness()`) and `/readyz` (canary fresh **and** a `SELECT 1` against
  the metadata pool succeeds within a timeout). Keep `/health` as the existing dumb 200 for LB compat.
- Keep the canary's 60-tick heartbeat INFO line — its *absence* in logs is unambiguous evidence the
  canary loop itself is parked (vs. the log writer being blocked).

**Done when:** killing the runtime in a test (block all workers) flips `/livez` to 503 within the
staleness budget while a normal slow handler leaves it at 200.

## Phase B — Task watchdog (wrap every eternal task)

**Port:** `rubix/crates/rubix-agent/src/boot/task_watchdog.rs` (~75 lines) verbatim — it's
self-contained, zero-dependency, zero runtime cost when the task runs forever.

`watch(label, handle) -> JoinHandle<()>` wraps a `JoinHandle<()>` in a second task that awaits it and
emits exactly one ERROR line when the inner task ends, with a stable `watcher=<label>` tag and an
`outcome` discriminant:

- `outcome=returned` — a supposed-to-be-infinite `loop {}` returned cleanly → a bug.
- `outcome=panicked` — task panicked (the panic hook logged the payload; this is the "supervisor
  noticed" half).
- `outcome=cancelled` — `.abort()` called; only valid during shutdown, a bug mid-run.
- `outcome=unknown-join-error` — unrecognised `JoinError`.

### Wiring into nexus

The three `nexus_api::…::spawn(state)` calls in `main.rs` currently return `()` (fire-and-forget).
Two options:

- **Minimal:** have each `spawn()` return its `JoinHandle<()>`, then in `main.rs`:
  `let _alert = task_watchdog::watch("alert_scheduler", alerting::schedule::spawn(state.clone()));`
  and the same for `detection_scheduler`, `changelog_prune`, and the canary tick.
- The `let _x = …` leak pattern is preserved — same lifetime semantics, just observable death.

Adopt a grep convention in the runbook: `target=nexus.task_watchdog watcher=alert_scheduler` finds a
dead scheduler instantly.

**Done when:** a test that panics inside a watched task produces exactly one
`target=nexus.task_watchdog … outcome=panicked` ERROR line.

## Phase C — Watchdog supervisor binary (do last, gate on prod)

**Port:** `rubix/crates/rubix-watchdog/` as a new `nexus-watchdog` crate (~430 lines).

A **separate binary**, not a thread — the whole point is that defense living *inside* a wedged tokio
runtime cannot fire, so the supervisor must be an OS-level peer. It probes `/livez` on an interval and,
on sustained failure, escalates with forensics:

1. **SIGUSR1** — last-gasp metrics dump; 2s grace; re-probe (fast-recovery check before destroying).
2. **SIGABRT** — core dump (preferred over SIGKILL so a post-mortem `gdb` is possible); 5s grace.
3. **SIGKILL** — final resort.
4. **restart** — via the configured restart command.

Design constraints to preserve from rubix:

- Single-threaded tokio runtime, **zero shared fate** with `nexus-api` — a build break in the server
  crate must not break the supervisor.
- **Everything env-driven** (probe URL, intervals, thresholds, restart command) for dev/prod parity —
  see `rubix/crates/rubix-watchdog/src/main.rs:102-155`.
- This composes with the existing memory note on the rubix supervisor process-group reaper (prevents
  child/grandchild leaks via `killpg` + boot pidfile reaper) — if nexus ever spawns extension child
  processes that outlive a crash, fold that in here too.

**Gate:** only worth standing up once nexus runs in prod under real load. Phases A+B deliver most of
the value (visibility) for ~200 lines; Phase C is the actuator and can follow.

## Build order & sizing

| Phase | What | Source | LOC | Risk |
|-------|------|--------|-----|------|
| A | runtime canary + `/livez` `/readyz` | `rubix-agent/src/boot/runtime_canary.rs` | ~123 | low — additive route + AppState field |
| B | task watchdog on the 3 schedulers + canary | `rubix-agent/src/boot/task_watchdog.rs` | ~75 | low — change `spawn()` return type |
| C | external supervisor binary | `rubix-watchdog/` | ~430 | med — new crate, signal handling, deploy change |

A and B are a single afternoon and isolated to `nexus-api`. Land them together; defer C behind a prod
decision.

## Non-goals

- Distributed tracing / OpenTelemetry — separate concern, belongs with WS-09 observability, not here.
- Per-flow CPU/memory quotas — the engine already has row/byte caps (`QueryGuards`); resource quotas
  are a `nexus-engine` topic, not an operational-floor one.
- Circuit breakers / bulkheads for datasource cascades — real, but a distinct WS; this one is purely
  about *detecting and recovering from a wedged process*, not shaping load.
