## Done

- Verified Group B work (B7a RUST_LOG honoring, B7b RpcError::internal_from_source + dispatch warn!) is already in HEAD via master commit b72f238; working tree clean.
- `cargo test -p starter-observability -p starter-mcp --lib --tests` green (17/17 starter-mcp, 2/2 starter-observability).
- `./rubix/scripts/lint-doc-refs.sh` clean.
- Smoke note rows B7a/B7b (lines ~557–558) + touched-files block (697–698) already document the change — no body edit needed.
- Recorded stage handover under runs/.../stage-2-group-b/handover.md, committed with the required `fix(starter-observability+starter-mcp) surface real error chain on -32603` title, and pushed `codeless/rubix-smoke-followups` (cfb0d52 → e7b97a5).

## Next

- Stage 3 picks up Group C; likely same de-facto-landed situation. Then proceed to the net-new follow-ups: B9 CH database routing, B10 stale handover volume names, N4 dead-code warning, alert-path integration test.

## What you need to know

- Same root cause as Stage 0/1: pre-existing master commit b72f238 ("migration of ui theme to starter") bundled all Group A+B+C smoke follow-ups with an unrelated UI-theme migration. Branch forked from master *after* b72f238, so the code is in ancestry — there is no working-tree diff to split. Splitting would require force-pushing master (forbidden).
- HEAD now in sync with origin/codeless/rubix-smoke-followups.

## Open questions

- Confirm with operator whether remaining "split into commits" stages should be no-op handover markers and effort should jump to the genuinely-new B9/B10/N4/alert-path follow-ups.
