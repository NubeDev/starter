## Done

- Added `crates/starter-flow-surfaces/src/clock.rs` with `Clock` trait + `SystemClock` + `TestClock` (Arc-shared Mutex<DateTime<Utc>>, `set`/`advance`/`epoch` helpers).
- Added `crates/starter-flow-surfaces/src/service.rs` with scheduler-flavored `FlowAsService` holding `Pool` + `Arc<FlowRegistry>` + `Arc<dyn FlowRunner>` + `Arc<dyn Clock>`, plus local `FlowRunner` trait and `ServiceError`. Implements `register_schedule` (INSERT … ON CONFLICT (tenant_id, flow_id) DO UPDATE), `unregister_schedule` (soft-disable via `enabled=FALSE`), `lookup_schedule`. Cron validated via `starter_cron::next_fire` before SQL.
- Wired `pub mod clock; pub mod service;` in `lib.rs` (no crate-root re-export, to avoid colliding with the existing event-driven `FlowAsService`).
- Cargo: added `starter-cron`, `starter-store-postgres`, sqlx (postgres/macros/chrono/uuid), chrono, uuid, ulid as deps; `starter-store-postgres` (testing) + sqlx (runtime-tokio) as dev-deps. Promoted `starter-cron` to `workspace.dependencies`.
- Tests: `tests/clock_test.rs` (6 unit cases — all pass via `cargo test -p starter-flow-surfaces --test clock_test`). `tests/register_test.rs` (4 testcontainers-gated `#[ignore]` cases — compile clean).
- Committed as `68b60c6 phase B.1 — FlowAsService scaffold — feat(starter-flow-surfaces) FlowAsService register/unregister + Clock`.

## Next

- Stage 6 (Phase B.2): `tick` + `start` on `service::FlowAsService` — `SELECT … FOR UPDATE SKIP LOCKED LIMIT 32` claim loop, dispatch via `FlowRunner::run`, write `last_run_*`, recompute `next_run_at` via `starter_cron::next_fire`, spawn 60s loop driven by the `Clock`. Then `tests/scheduled_flows_tick_test.rs`.

## What you need to know

- Naming collision intentional: the crate root keeps the existing event-driven `pub struct FlowAsService` (broadcast-subscriber). The new scheduler-flavored one is reachable only as `starter_flow_surfaces::service::FlowAsService`. Module doc on `service.rs` calls this out. Don't `pub use` it at the crate root.
- `FlowRunner` is a NEW trait local to `starter-flow-surfaces::service` — NOT the concrete `starter_flow::run::FlowRunner` struct already used inside `lib.rs`. Adapter from the concrete runner to this trait will be a stage-6 (or rubix-agent) concern.
- `register_schedule` uses `ulid::Ulid::new().to_string()` for the TEXT PK (matches the schema's `Stored as TEXT to stay portable with the sqlite twin` comment).
- `created_by` defaults to `Uuid::nil()` (matches sentinel actor used in the existing `scheduled_flows.rs` integration test).
- `register_test.rs` cron `"0 0 8 * * MON"` is the 6-field form `starter-cron` expects (the migration test uses 5-field `"0 8 * * 1"` which only works because that test never re-parses through `starter_cron`; do NOT copy the 5-field form into surfaces tests).

## Open questions

- Whether `FlowRunner` should ultimately live in `starter-flow-spi` instead of `starter-flow-surfaces` (so rubix-agent doesn't have to depend on surfaces just to implement the trait). Defer to stage 6 review.
- Whether the crate-root `FlowAsService` should be renamed to `EventDrivenFlowAsService` to remove the ambiguity for good; out of scope this stage.
