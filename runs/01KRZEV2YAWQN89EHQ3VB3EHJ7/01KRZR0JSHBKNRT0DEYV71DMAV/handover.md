## Done

- Replaced `ContributeWorker { cron }` with `{ interval_seconds, jitter_seconds, on_error: OnErrorPolicy{retry, initial_backoff_ms, max_backoff_ms, max_attempts} }` in `starter-ext-spi`; updated SCOPE-example manifest test; re-exported new types from `starter-ext-spi` and `starter-ext-sdk`.
- New crate `starter-ext-workers` (added to workspace members + workspace deps): one task per worker, no shared queue, no fan-out. `WorkersScheduler::new(registry, dispatcher).start(SchedulerOptions)` returns a cheap-clone `WorkersSchedulerHandle`.
- `WorkerDispatcher` trait + `BuiltinWorkerDispatcher` (via `BuiltinWorkerRegistry`, blocking-pool execution, tokio timeout) + `ProcessWorkerDispatcher` / `WasmWorkerDispatcher` v0.1 stubs that carry `request_timeout` so the JSON-RPC slice lands additively.
- `WorkerState { worker_id, extension_id, status, last_run, last_error, next_due, attempt, total_runs }` with RFC-3339 serialisation and `WorkerStatus { Healthy, BackingOff, Stopped }`. `WorkerStateSource` trait surfaces snapshots.
- Scheduler logic: on success → `Healthy`, `attempt=0`, next_due = now + interval + uniform(0..jitter); on `Err` → record `last_error`, `attempt += 1`, `BackingOff` with exponential backoff `min(max_backoff_ms, initial * 2^(attempt-1))`; on `attempt >= max_attempts` or `retry: never` or fatal-config error → `Stopped, next_due = None`.
- Testing seam: `WorkersSchedulerHandle::tick_now(&ext, worker_id)` (per-worker `tokio::sync::Notify`); `snapshot_for` / `snapshot_one` / `shutdown`.
- Worker state surfaced on `GET /extensions/<id>`: `ExtensionDetail` gains `workers: Vec<Value>`; `ExtensionAdminBuilder::with_worker_states_fn(...)` is the seam (closure-based so `ext-server` does not depend on `ext-workers`).
- Tests: 8 unit + integration tests in `starter-ext-workers` (success path, max-attempts → Stopped, success-after-failure resets attempt, builtin dispatcher round-trip + timeout, NotWired stub, fatal-config classification, RFC-3339 serialisation). All workspace crates I touched compile and test green: `cargo test -p starter-ext-spi -p starter-ext-server -p starter-ext-workers` → 52 passing.

## Next

- Stage 16 / final stage in the 16-stage plan (next session picks it up — likely the v0.1 wrap-up: smoke-test orchestration, examples gardening, or SCOPE close-out).

## What you need to know

- I did not add an `examples/hello-worker` example — none of the prior adapter stages (REST, CLI) added their own hello-* if there was no clean way to flip a single feature on an existing example; the scheduler is exercised end-to-end through the in-crate tests instead.
- Pre-existing issue (not introduced by this stage): `cargo check --workspace` fails because the `__STARTER_EXT_FLAVOUR_MARKER` mutually-exclusive guard in `starter-ext-sdk` trips when cargo unifies features across `examples/hello-{builtin,process,wasm}`. Use per-crate `cargo check -p ...` like the prior stages did.
- `WorkerStatesFn` is `Arc<dyn Fn(&ExtensionId) -> Vec<serde_json::Value> + Send + Sync>` — consumers wire it as `.with_worker_states_fn({ let h = handle.clone(); move |id| h.snapshot_for(id).iter().map(|s| serde_json::to_value(s).unwrap()).collect() })`. Keeps `starter-ext-server` independent of `starter-ext-workers`.
- `WorkerError::NotFound` / `Forbidden` are classified as `is_fatal_config()` → immediate `Stopped`, bypassing `max_attempts` (the user has to fix configuration anyway).
- RFC-3339 timestamp formatting is inlined (Howard Hinnant civil_from_days, ~10 LoC) to avoid pulling in `chrono` / `time` for one field.

## Open questions

- (none)
