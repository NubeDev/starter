## Done

- Added `InsightsConfig { disk_warn_threshold: u8 = 90 }` nested under `AgentConfig` (rubix-agent/src/boot/config.rs)
- Documented the new `[insights]` section (commented) in rubix/dev/agent.toml
- Added `DiskTool::with_insights_threshold(u8)` + field; default = `INSIGHTS_DISK_ALERT_THRESHOLD` (90)
- `run_insights_gate(response, threshold)` — threshold is now a parameter; the `> 90` literal is gone
- `registry::build_tool_registry(ch, insights_disk_threshold)` threads cfg into the disk tool; main.rs + boot::mcp::register pass `cfg.insights.disk_warn_threshold`
- New integration test `rubix/crates/rubix-agent/tests/alert_path_threshold_test.rs` (4 tests, all pass): default=90, fires at 60% vs threshold 50, silent at 60% vs threshold 70, registry plumbing canary. Header justifies the in-memory `alert_send::dispatched_count()` sink choice over log capture.
- Updated existing rubix-tools insights test + 2 rubix-agent tests for the new `build_tool_registry` / `run_insights_gate` signatures
- `cargo test -p rubix-tools -p rubix-agent` green
- Committed as `test(rubix-agent) alert path verification via threshold parameterisation` (f29703a) and pushed

## Next

- Stage 8 (final): the gate stage that opens the PR — see prior stage 3 notes about `gh pr create --base master --head codeless/rubix-smoke-followups`. Branch is now 7 commits ahead of master post-merge.

## What you need to know

- The refactor came in under the 100-LoC BLOCKED ceiling — no need to raise.
- `alert_send::dispatched_count()` is process-wide; the new tests use deltas, not absolutes, so they coexist with the existing `rubix-tools/tests/system_disk_insights_test.rs` cases.
- Two unrelated test files (`changelog_middleware_test.rs`, `rest_disk_test.rs`) needed signature updates to `build_tool_registry(None, 90)` — pure mechanical, no semantic change.
- `RUBIX_INSIGHTS__DISK_WARN_THRESHOLD` works automatically via the existing loader's `with_env("RUBIX_")` + double-underscore nesting convention.

## Open questions

- (none)
