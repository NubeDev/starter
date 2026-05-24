## Done

- Reviewed Phase D commits 8fd1fb6 (D.1 YAML rewrite + stub deletion), 786e7e7 (D.2 rubix-agent boot wires FlowAsService), f3abb59 (D.3 goal_6 test + design doc).
- Ran `cargo test -p rubix-agent --test goal_6_weekly_report_test -- --ignored` against local Docker — 1 passed in 5.66s.
- Verified R1 (no starter-* crate depends on rubix-*), R2 (no transport changes), R4/R5 (trust boundary untouched), and no wire-format files (openapi/proto/schema) modified in Phase D.

## Next

- (none) — gate FAILed; do not start Phase E. The fix belongs in a subsequent WORK stage: either change `rubix/crates/rubix-flows/flows/weekly-report.yaml` line 10 from `"0 8 * * 1"` to a 6-field form (e.g. `"0 0 8 * * MON"`), or upgrade `starter-cron` to accept 5-field POSIX cron and normalise internally. Whichever path, re-verify by booting rubix-agent and confirming the seeder logs `cron_expr=…` without error, then run the curl manual flow end-to-end.

## What you need to know

- The goal_6 integration test passes because it bypasses the boot seeder — it hand-builds a 6-field cron in-test and calls `register_schedule` directly. Bundled YAML never traverses `starter_cron::next_fire` in that path, so the test is green while production boot would fail.
- Evidence sites: `rubix/crates/rubix-flows/flows/weekly-report.yaml:10`, `crates/starter-cron/src/lib.rs:17` (grammar doc), `crates/starter-flow-surfaces/src/service.rs:191` (up-front validation), `rubix/crates/rubix-agent/src/boot/scheduler.rs:119` (seeder propagates the error).
- Prior session's handover already raised this exact mismatch as the gate's open question.

## Open questions

- Pick the fix path: rewrite YAML to 6-field (cheapest, one-line) vs. teach starter-cron to accept 5-field POSIX (matches what most operators will type, but widens the parser surface). Recommendation: 6-field YAML now + a follow-up to accept 5-field with documented normalisation rules.

FAIL: bundled weekly-report.yaml declares a 5-field cron that starter-cron's 6/7-field grammar rejects, so the rubix-agent boot seeder errors before FlowAsService can fire — the gate's required boot→fire→blob→undo manual flow cannot run end-to-end.
