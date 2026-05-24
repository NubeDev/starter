## Done

- Added `SchedulerConfig { enabled, tick_interval_seconds }` to `AgentConfig` with defaults `true` / `60` in `rubix/crates/rubix-agent/src/boot/config.rs`.
- Added top-level `cron_expr: Option<String>` to `RubixFlowYaml` so the boot seeder can read the weekly-report schedule.
- New `rubix/crates/rubix-agent/src/boot/scheduler.rs`: constructs `FlowAsService` (PG pool + fresh `FlowRegistry` + `SystemClock` + `ToolRegistryRunner` dispatching via MCP `ToolRegistry`), seeds `starter_scheduled_flows` from every bundled YAML carrying `trigger: schedule` + `cron_expr` (idempotent — `register_schedule` upserts), and calls `FlowAsService::start`.
- Wired into `main.rs` after MCP surface build; handle leaked into process lifetime like `_undo_sweep`. Skips when no PG DSN or `[scheduler].enabled = false`.
- `cargo build -p rubix-agent` + lib tests green.
- Committed as `786e7e7` with message starting "stage 12: phase D.2 — rubix-agent boot wires FlowAsService".

## Next

- (none) — next session picks up stage 13.

## What you need to know

- `tick_interval_seconds` config field is parsed but not yet honored — `FlowAsService::start` is hardcoded to 60s upstream; the field is documented as forward-compat. If the next stage wants real override, it must extend `starter-flow-surfaces::service` with a `start_with_interval` variant.
- The scheduler uses a fresh empty `FlowRegistry` (not the MCP one) because dispatch goes through `ToolRegistryRunner` → MCP `ToolRegistry.get(flow_id).invoke({})`. Tool id equals flow id for `FlowAsTool` wrappers (see `rubix-agent/src/boot/mcp/register.rs`).
- Seeder bails if a flow declares `trigger: schedule` without `cron_expr` — surfaces authoring errors at boot.
- `SYSTEM_TENANT` (all-zero UUID) reused from `flows_seed` for scheduler tenancy.

## Open questions

- Drop-schedules-for-removed-flows is not implemented this stage; a bundled YAML deletion leaves a stale row enabled. SCOPE doesn't pin this for D.2 but the gate may flag it.
