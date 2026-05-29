## Done

- New leaf crate `starter-ext-metrics` (`crates/starter-ext-metrics`): `MetricsRegistry` over `DashMap<ExtensionId, Arc<Counters>>` with atomic increments, `CounterSnapshot`, `ProcessGauges`, and the `merged()` projection. Added to workspace members + `[workspace.dependencies]` (dashmap 6) and to the per-crate CI sweep.
- `ExtensionMetrics` aggregate shape added to `starter-ext-spi/src/metrics.rs` (re-exported), covering process/lifecycle_state/restarts/capability_violations/tool_calls/tool_errors/rest_requests/worker_runs/worker_failures/events_dropped.
- mcp adapter: both tool bindings carry an optional `Counters` handle, bump tool_calls/tool_errors on invoke; `register_tools_with_metrics` / `register_process_tools_with_metrics` + a `metrics` param threaded through the `_with_engine` fns.
- server REST dispatch: `HandlerSpec` carries optional counters, bumps `rest_requests` in non_streaming/sse/ndjson; `RestRouterOptions.metrics` knob.
- workers: `WorkersScheduler::with_metrics(...)` bumps worker_runs/worker_failures per run.
- supervisor: `SupervisorHandle::{restarts_total, events_dropped, lifecycle_state}` + `EventRing::dropped()` for the process gauges.
- `GET /extensions/{id}/metrics` (gated, Role::Admin) via new `metrics.rs`; `ExtensionAdmin::with_metrics` + `metrics()` accessor (defaults to a fresh empty registry).
- Tests: metrics crate (counter increments + merged projection), server admin_routes (merged endpoint w/o supervisor + 404), workers (run/failure counting). Build + clippy -D warnings + fmt green per-crate.

## Next

- Stage 4: data cleanup on uninstall (warehouse tables, enablement row, UI/sidebar cache, skills) — `CleanupItem`/`CleanupKind` trait in starter-ext-server, providers in rubix.
- Later (stage 5/6): rubix must actually pass one shared `MetricsRegistry` into the mcp register fns (`register_*_with_metrics`), `RestRouterOptions.metrics`, `WorkersScheduler::with_metrics`, AND `ExtensionAdmin::with_metrics` — same handle everywhere — and project `/metrics` into the admin envelope/UI.

## What you need to know

- The starter-extensions workspace does NOT build with `cargo build --workspace` (pre-existing: hello-builtin vs hello-wasm flavour markers collide under feature unification). CI uses a per-crate `-p` sweep — see `.github/workflows/starter-extensions.yml`. Verify with `cargo test -p <crate>`, not `--workspace`.
- All adapter wiring is additive/optional: `None` ⇒ zero metrics overhead, so existing callers and TestApps are unaffected. The `_with_engine` mcp fns gained a `metrics: Option<&MetricsRegistry>` param (no in-repo external callers; rubix uses the unchanged wrappers).
- `restarts_total` is derived from retained `RestartScheduled` ring events (matches existing `restart_count` on `/extensions/{id}`); `events_dropped = next_seq - len`.

## Open questions

- (none)
