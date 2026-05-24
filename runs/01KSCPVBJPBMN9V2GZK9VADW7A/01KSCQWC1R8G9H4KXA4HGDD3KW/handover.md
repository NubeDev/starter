## Done

- Reviewed Phase B diff (commits 853fc41 workspace bootstrap + 5536995 README/CI).
- Confirmed rubix/extensions sibling workspace builds; example binary present at rubix/extensions/target/debug/rubix-example-extension; CI job added.
- Confirmed Layer-1 invariants hold: R1 dep direction (com.rubix.example → starter-ext-sdk via path, no reverse edges), R2 single transport (no transport changes), R4/R5 trust boundary untouched, no wire-format changes.
- OQ-3 evidence captured: rubix/extensions/com.rubix.example/ contains block.yaml, flows/, kinds/, process/, skills/, README.md — no ui/ directory and no .tsx files, so the example does NOT ship UI assets today.
- Committed empty gate marker as stage 6.

## Next

- (none) — fresh session picks up Phase C (next WORKFLOW item, likely host wire-in into rubix-agent).

## What you need to know

- PASS: both workspaces build green, example process binary present, no UI assets in com.rubix.example (Phase D will add ui/main.tsx); R1/R2/R4/R5 + wire-formats untouched.
- Binary path is rubix/extensions/target/debug/rubix-example-extension (sibling-workspace target dir), which matches the stage's intent even though the stage text wrote "target/debug/" without the workspace prefix.
- Phase D is now confirmed to need a minimal ui/main.tsx contribution from com.rubix.example for the frontend round-trip test.

## Open questions

- (none)
