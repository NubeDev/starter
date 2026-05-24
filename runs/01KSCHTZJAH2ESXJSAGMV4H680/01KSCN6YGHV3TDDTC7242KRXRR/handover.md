## Done

- Added rubix/crates/rubix-agent/tests/goal_6_weekly_report_test.rs covering the full Goal 6 loop: PG (scheduled_flows) + CH (system_disk_history) + tempdir FsBlobStore via testcontainers, 7-day seed, TestClock +7d → one tick fires, scheduled_flows row lands as `succeeded` with re-armed next_run_at, agent reply non-empty, rendered html blob contains all seven seeded peaks, `rubix.undo.last` deletes the blob.
- Added rubix-agent dev-dependencies: starter-blob-fs, bytes, tempfile.
- Rewrote rubix/docs/design/reports/README.md from placeholder to present-tense with pipeline diagram, verb contracts, scheduling pointer, and test-coverage matrix.
- `cargo test -p rubix-agent --test goal_6_weekly_report_test --no-run` compiles cleanly.
- Committed as `stage 13: phase D.3 — Goal 6 integration test + design doc — test(rubix-agent) + docs(design) goal 6 end-to-end`.

## Next

- Stage 14: per the rubix-goal-6 SCOPE, next is the Phase D gate (D.3 landed) — confirm two D-phase commits, two `-- --ignored` testcontainer runs (the analytics_report_test from stage 9 and the new goal_6_weekly_report_test) pass, and the rubix-agent boots without panic with the scheduler enabled.

## What you need to know

- The new test does not stand up the ai-agent loop; it uses a stub `AnalyticsReportRunner` `FlowRunner` that dispatches `analytics.report` through `UndoDispatcher` directly. This mirrors goal_3/goal_4 patterns and is honest because `build_tool_registry` does not yet expose the analytics verbs (rubix-agent/src/registry.rs only wires disk/db/flow_errors/alert/dashboard). When analytics tools are registered there the stub can be deleted in favour of the production `ToolRegistryRunner`.
- Cron expression in the test is the 6-field `0 0 8 * * MON` because `starter-cron` wraps the `cron` crate which requires the 6/7-field grammar. The bundled weekly-report YAML still carries the 5-field `0 8 * * 1` — that 5-field grammar will fail `starter_cron::next_fire` if it reaches the seeder unchanged. Worth verifying in the gate.
- The test is `#[ignore]`d (Docker required) — invoke with `cargo test -p rubix-agent --test goal_6_weekly_report_test -- --ignored`.

## Open questions

- Does the 5-field cron in `rubix/crates/rubix-flows/flows/weekly-report.yaml` actually parse via `starter_cron::next_fire`? If not, stage 14 should either upgrade `starter-cron` to accept 5-field or rewrite the YAML to 6-field.
