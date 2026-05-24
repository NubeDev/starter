## Done

- Implemented `TriggerSchedule` NodeBehavior in `crates/starter-flow-nodes/src/trigger_schedule.rs`: input config slot `cron_expr` (String) → output slot `schedule` (String), with `TriggerScheduleSettings` schemars schema, typed errors (`MissingCronExpr`, `InvalidCronExprType`), R12 span, R13 cancel check
- Added 4 unit tests (happy path, missing, wrong type, cancelled)
- Added `crates/starter-flow-nodes/tests/trigger_schedule_test.rs` with 2 integration tests: kind registers in `StaticNodeKindRegistry::with_builtins`, and a one-node flow rooted at `trigger.schedule` driven via real `propagator` + `InMemoryGraphStore` confirms `cron_expr="0 0 * * 0"` passes through unmodified on the `schedule` slot (asserted via both `FlowEvent::NodeEmitted` and `store.read_slot`)
- `cargo test -p starter-flow-nodes --features all-kinds` green; committed as 3f8c109

## Next

- Stage 4 of 15 per SCOPE/WORKFLOW — next session picks it up (likely Phase B: FlowAsService schedule trigger wiring on top of `starter-cron` + `starter_scheduled_flows`)

## What you need to know

- Input slot constant: `CRON_EXPR_SLOT = "cron_expr"`. Output slot constant: `SCHEDULE_SLOT = "schedule"`. Distinct names so the trigger doesn't re-fire itself when the propagator writes the output through the R2 chokepoint
- Body is sync (no `.await`); `trigger-schedule` feature stays `[]` — no `dep:tokio` needed (dev-dep tokio covers tests)
- The body does NOT validate the cron expression — validation lives in `starter-cron::next_fire` per the comment, so a bad cron string flows through and surfaces only at the scheduler tick
- i18n catalog keys for `starter.flow.node.trigger-schedule.{label,summary,help}` already exist in EN + ES (added pre-stage)

## Open questions

- (none)
